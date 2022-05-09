use std::{iter::Iterator, time::Duration};

use async_recursion::async_recursion;
use regex::RegexBuilder;
use snafu::ResultExt;
use tokio::time::delay_for;

use crate::{
	core::{get_commit_checks, get_commit_statuses, AppState, Status},
	error::*,
	github::*,
	merge_request::{
		check_all_mergeability, check_mergeability, cleanup_merge_request,
		handle_merged_pull_request, queue_merge_request, MergeRequest,
		MergeRequestCleanupReason, MergeRequestDependency,
		MergeRequestQueuedMessage,
	},
	shell::*,
	types::Result,
	COMPANION_LONG_REGEX, COMPANION_PREFIX_REGEX, COMPANION_SHORT_REGEX,
	OWNER_AND_REPO_SEQUENCE, PR_HTML_URL_REGEX,
};

#[derive(Clone)]
pub struct CompanionReferenceTrailItem {
	pub owner: String,
	pub repo: String,
}

async fn update_companion_pr_branch(
	state: &AppState,
	comp_pr: &GithubPullRequest,
	dependencies_to_update: &Vec<MergeRequestDependency>,
) -> Result<String> {
	let AppState {
		gh_client, config, ..
	} = state;
	// Constantly refresh the token in-between operations, preferably right before
	// using it, for avoiding expiration issues. Some operations such as cloning
	// repositories might take a long time, thus causing the token to be
	// invalidated after it finishes. In any case, the token generation API should
	// backed by a cache, thus there's no problem with spamming the refresh calls.

	let owner = &comp_pr.base.repo.owner.login;
	let owner_repo = &comp_pr.base.repo.name;
	let contributor = &comp_pr.head.repo.owner.login;
	let contributor_repo = &comp_pr.head.repo.name;
	let contributor_branch = &comp_pr.head.ref_field;
	let number = comp_pr.number;

	let repo_dir = config.repos_path.join(owner_repo);
	let repo_dir_str = if let Some(repo_dir_str) = repo_dir.as_os_str().to_str()
	{
		repo_dir_str
	} else {
		return Err(Error::Message {
			msg: format!(
				"Path {:?} could not be converted to string",
				repo_dir
			),
		});
	};

	if repo_dir.exists() {
		log::info!("{} is already cloned; skipping", owner_repo);
	} else {
		let token = gh_client.auth_token().await?;
		let secrets_to_hide = [token.as_str()];
		let secrets_to_hide = Some(&secrets_to_hide[..]);
		let owner_repository_domain =
			format!("github.com/{}/{}.git", owner, owner_repo);
		let owner_remote_address = format!(
			"https://x-access-token:{}@{}",
			token, owner_repository_domain
		);
		run_cmd_in_cwd(
			"git",
			&["clone", "-v", &owner_remote_address, repo_dir_str],
			CommandMessage::Configured(CommandMessageConfiguration {
				secrets_to_hide,
				are_errors_silenced: false,
			}),
		)
		.await?;
	}

	// The contributor's remote entry might exist from a previous run (not expected for a fresh
	// clone). If that is the case, delete it so that it can be recreated.
	if run_cmd(
		"git",
		&["remote", "get-url", contributor],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide: None,
			are_errors_silenced: true,
		}),
	)
	.await
	.is_ok()
	{
		run_cmd(
			"git",
			&["remote", "remove", contributor],
			&repo_dir,
			CommandMessage::Configured(CommandMessageConfiguration {
				secrets_to_hide: None,
				are_errors_silenced: false,
			}),
		)
		.await?;
	}

	let contributor_remote_branch =
		format!("{}/{}", contributor, contributor_branch);
	let token = gh_client.auth_token().await?;
	let secrets_to_hide = [token.as_str()];
	let secrets_to_hide = Some(&secrets_to_hide[..]);
	let contributor_repository_domain =
		format!("github.com/{}/{}.git", contributor, contributor_repo);
	let contributor_remote_address = format!(
		"https://x-access-token:{}@{}",
		token, contributor_repository_domain
	);

	run_cmd(
		"git",
		&["remote", "add", contributor, &contributor_remote_address],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: false,
		}),
	)
	.await?;
	run_cmd(
		"git",
		&["fetch", contributor, contributor_branch],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: false,
		}),
	)
	.await?;

	// The contributor's branch might exist from a previous run (not expected for a fresh clone).
	// If so, delete it so that it can be recreated.
	// Before deleting the branch, it's first required to checkout to a detached SHA so that any
	// branch can be deleted without problems (e.g. the branch we're trying to deleted might be the
	// one that is currently active, and so deleting it would fail).
	let head_sha_output = run_cmd_with_output(
		"git",
		&["rev-parse", "HEAD"],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: false,
		}),
	)
	.await?;
	run_cmd(
		"git",
		&[
			"checkout",
			String::from_utf8(head_sha_output.stdout)
				.context(Utf8)?
				.trim(),
		],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: true,
		}),
	)
	.await?;
	let _ = run_cmd(
		"git",
		&["branch", "-D", contributor_branch],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: true,
		}),
	)
	.await;
	run_cmd(
		"git",
		&["checkout", "--track", &contributor_remote_branch],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: false,
		}),
	)
	.await?;

	let owner_remote = "origin";
	let owner_branch = "master";
	let owner_remote_branch = format!("{}/{}", owner_remote, owner_branch);

	let token = gh_client.auth_token().await?;
	let secrets_to_hide = [token.as_str()];
	let secrets_to_hide = Some(&secrets_to_hide[..]);
	let owner_repository_domain =
		format!("github.com/{}/{}.git", owner, owner_repo);
	let owner_remote_address = format!(
		"https://x-access-token:{}@{}",
		token, owner_repository_domain
	);
	run_cmd(
		"git",
		&["remote", "set-url", owner_remote, &owner_remote_address],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: false,
		}),
	)
	.await?;
	run_cmd(
		"git",
		&["fetch", owner_remote, owner_branch],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: false,
		}),
	)
	.await?;

	// Create master merge commit before updating packages
	let master_merge_result = run_cmd(
		"git",
		&["merge", &owner_remote_branch, "--no-ff", "--no-edit"],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: false,
		}),
	)
	.await;
	if let Err(e) = master_merge_result {
		log::info!("Aborting companion update due to master merge failure");
		run_cmd(
			"git",
			&["merge", "--abort"],
			&repo_dir,
			CommandMessage::Configured(CommandMessageConfiguration {
				secrets_to_hide,
				are_errors_silenced: false,
			}),
		)
		.await?;
		return Err(e);
	}

	log::info!(
		"Dependencies to update for {}/{}/pull/{}: {:?}",
		owner,
		owner_repo,
		number,
		dependencies_to_update
	);
	for dependency_to_update in dependencies_to_update {
		let source_to_update = format!(
			"{}/{}/{}{}",
			config.github_source_prefix,
			dependency_to_update.owner,
			dependency_to_update.repo,
			config.github_source_suffix
		);
		log::info!(
			"Updating references of {} in the Cargo.lock of {:?}",
			source_to_update,
			repo_dir
		);
		run_cmd(
			"reref",
			&[
				"--match-git",
				&source_to_update,
				"--remove-field",
				"branch",
				"--add-field",
				"rev",
				"--added-field-value",
				&format!("refs/pulls/{}/head", dependency_to_update.number),
				"--autocommit",
				"--require-removed-field",
			],
			&repo_dir,
			CommandMessage::Configured(CommandMessageConfiguration {
				secrets_to_hide: None,
				are_errors_silenced: false,
			}),
		)
		.await?;
	}

	run_cmd(
		"git",
		&["push", contributor, contributor_branch],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: false,
		}),
	)
	.await?;

	log::info!(
		"Getting the head SHA after a PR branch update in {}",
		&contributor_remote_branch
	);
	let updated_sha_output = run_cmd_with_output(
		"git",
		&["rev-parse", "HEAD"],
		&repo_dir,
		CommandMessage::Configured(CommandMessageConfiguration {
			secrets_to_hide,
			are_errors_silenced: false,
		}),
	)
	.await?;
	let updated_sha = String::from_utf8(updated_sha_output.stdout)
		.context(Utf8)?
		.trim()
		.to_string();

	Ok(updated_sha)
}

fn parse_companion_from_url(
	body: &str,
) -> Option<PullRequestDetailsWithHtmlUrl> {
	parse_companion_from_long_url(body)
		.or_else(|| parse_companion_from_short_url(body))
}

fn parse_companion_from_long_url(
	body: &str,
) -> Option<PullRequestDetailsWithHtmlUrl> {
	let re = RegexBuilder::new(COMPANION_LONG_REGEX!())
		.case_insensitive(true)
		.build()
		.unwrap();
	let caps = re.captures(body)?;
	let html_url = caps.name("html_url")?.as_str().to_owned();
	let owner = caps.name("owner")?.as_str().to_owned();
	let repo = caps.name("repo")?.as_str().to_owned();
	let number = caps
		.name("number")?
		.as_str()
		.to_owned()
		.parse::<i64>()
		.ok()?;
	Some(PullRequestDetailsWithHtmlUrl {
		html_url,
		owner,
		repo,
		number,
	})
}

fn parse_companion_from_short_url(
	body: &str,
) -> Option<PullRequestDetailsWithHtmlUrl> {
	let re = RegexBuilder::new(COMPANION_SHORT_REGEX!())
		.case_insensitive(true)
		.build()
		.unwrap();
	let caps = re.captures(body)?;
	let owner = caps.name("owner")?.as_str().to_owned();
	let repo = caps.name("repo")?.as_str().to_owned();
	let number = caps
		.name("number")?
		.as_str()
		.to_owned()
		.parse::<i64>()
		.ok()?;
	let html_url = format!(
		"https://github.com/{owner}/{repo}/pull/{number}",
		owner = owner,
		repo = repo,
		number = number
	);
	Some(PullRequestDetailsWithHtmlUrl {
		html_url,
		owner,
		repo,
		number,
	})
}

pub fn parse_all_companions(
	companion_reference_trail: &[CompanionReferenceTrailItem],
	body: &str,
) -> Vec<PullRequestDetailsWithHtmlUrl> {
	body.lines()
		.filter_map(|line| {
			parse_companion_from_url(line).and_then(|comp| {
				// Break cyclical references between dependency and dependents because we're only
				// interested in the dependency -> dependent relationship, not the other way around.
				for item in companion_reference_trail {
					if comp.owner == item.owner && comp.repo == item.repo {
						return None;
					}
				}
				Some(comp)
			})
		})
		.collect()
}

#[async_recursion]
pub async fn check_all_companions_are_mergeable(
	state: &AppState,
	pr: &GithubPullRequest,
	requested_by: &str,
	companion_reference_trail: &[CompanionReferenceTrailItem],
) -> Result<bool> {
	let companions = match pr.parse_all_companions(companion_reference_trail) {
		Some(companions) => {
			if companions.is_empty() {
				return Ok(true);
			} else {
				companions
			}
		}
		_ => return Ok(true),
	};

	let AppState { gh_client, .. } = state;
	for PullRequestDetailsWithHtmlUrl {
		html_url,
		owner,
		repo,
		number,
	} in companions
	{
		let companion = gh_client.pull_request(&owner, &repo, number).await?;

		if companion.merged {
			continue;
		}

		let has_user_owner = companion
			.user
			.as_ref()
			.map(|user| user.type_field == GithubUserType::User)
			.unwrap_or(false);
		if !has_user_owner {
			return Err(Error::Message {
				msg: format!(
					"Companion {} is not owned by a user, therefore processbot would not be able to push the lockfile update to their branch due to a Github limitation (https://github.com/isaacs/github/issues/1681)",
					html_url
				),
			});
		}

		if !companion.maintainer_can_modify
			// Even if the "Allow edits from maintainers" setting is not enabled, as long as the
			// companion belongs to the same organization, the bot should still be able to push
			// commits.
			&& companion
				.head
				.repo
				.owner.login != pr.base.repo.owner.login
		{
			return Err(Error::Message {
				msg: format!(
					"Github API says \"Allow edits from maintainers\" is not enabled for {}. The bot needs that permission to update the PR's lockfile. Please check https://docs.github.com/en/github/collaborating-with-pull-requests/working-with-forks/allowing-changes-to-a-pull-request-branch-created-from-a-fork.",
					html_url
				),
			});
		}

		if !companion.mergeable.unwrap_or(false) {
			return Err(Error::Message {
				msg: format!("Companion {} is not mergeable", &html_url),
			});
		}

		let status_outcome = get_commit_statuses(
			state,
			&companion.base.repo.owner.login,
			&companion.base.repo.name,
			&companion.head.sha,
			&companion.html_url,
			true,
		)
		.await?;
		match status_outcome {
			Status::Success => (),
			Status::Pending => return Ok(false),
			Status::Failure => {
				return Err(Error::Message {
					msg: format!(
						"Companion {} has failed commit statuses",
						&companion.html_url
					),
				})
			}
		}

		let checks_outcome = get_commit_checks(
			&state.gh_client,
			&companion.base.repo.owner.login,
			&companion.base.repo.name,
			&companion.head.sha,
			&companion.html_url,
		)
		.await?;
		match checks_outcome {
			Status::Success => (),
			Status::Pending => return Ok(false),
			Status::Failure => {
				return Err(Error::Message {
					msg: format!(
						"Companion {} has failed checks",
						&companion.html_url
					),
				})
			}
		}

		// Keeping track of the trail of references is necessary to break chains like A -> B -> C -> A
		let next_companion_reference_trail = {
			let mut next_trail =
				Vec::with_capacity(companion_reference_trail.len() + 1);
			next_trail.extend_from_slice(companion_reference_trail);
			next_trail.push(CompanionReferenceTrailItem {
				owner: (&pr.base.repo.owner.login).into(),
				repo: (&pr.base.repo.name).into(),
			});
			next_trail
		};

		check_all_mergeability(
			state,
			&companion,
			requested_by,
			&next_companion_reference_trail,
		)
		.await?;
	}

	Ok(true)
}

#[async_recursion]
pub async fn update_companion(
	state: &AppState,
	comp: &MergeRequest,
	msg: &MergeRequestQueuedMessage,
) -> Result<Option<String>> {
	let AppState {
		gh_client, config, ..
	} = state;

	match async {
		let comp_pr = gh_client
			.pull_request(&comp.owner, &comp.repo, comp.number)
			.await?;
		if handle_merged_pull_request(state, &comp_pr, &comp.requested_by)
			.await?
		{
			return Ok(None);
		}

		check_mergeability(state, &comp_pr)?;

		let (updated_sha, comp_pr) = match comp.dependencies.as_ref() {
			Some(dependencies) if !dependencies.is_empty() => {
				log::info!(
					"Updating {} including the following dependencies: {:?}",
					comp_pr.html_url,
					dependencies
				);

				let updated_sha =
					update_companion_pr_branch(state, &comp_pr, dependencies)
						.await?;

				// Wait a bit for the statuses to settle after we've updated the companion
				delay_for(Duration::from_millis(
					config.companion_status_settle_delay,
				))
				.await;

				// Fetch it again since we've pushed some commits and therefore some status or check might have
				// failed already
				let comp_pr = gh_client
					.pull_request(
						&comp_pr.base.repo.owner.login,
						&comp_pr.base.repo.name,
						comp_pr.number,
					)
					.await?;

				// Sanity-check: the PR's new HEAD sha should be the updated SHA we just
				// pushed
				if comp_pr.head.sha != updated_sha {
					return Err(Error::HeadChanged {
						expected: updated_sha.to_string(),
						actual: comp_pr.head.sha,
					});
				}

				// Cleanup the pre-update SHA in order to prevent late status deliveries from
				// removing the updated SHA from the database
				cleanup_merge_request(
					state,
					&comp.sha,
					&comp.owner,
					&comp.repo,
					comp.number,
					Some(&MergeRequestCleanupReason::AfterSHAUpdate(
						&updated_sha,
					)),
				)?;

				(Some(updated_sha), comp_pr)
			}
			Some(_) | None => (None, comp_pr),
		};

		log::info!(
			"Companion updated; waiting for checks on {}",
			comp_pr.html_url
		);
		queue_merge_request(
			state,
			&MergeRequest {
				sha: comp_pr.head.sha,
				owner: comp_pr.base.repo.owner.login,
				repo: comp_pr.base.repo.name,
				number: comp_pr.number,
				html_url: comp_pr.html_url,
				requested_by: (&comp.requested_by).into(),
				// Set "was_updated: true" to avoid updating a branch more than once
				was_updated: true,
				dependencies: comp.dependencies.clone(),
			},
			msg,
		)
		.await?;

		Ok(updated_sha)
	}
	.await
	{
		Err(err) => Err(err.with_pull_request_details(PullRequestDetails {
			owner: comp.owner.to_owned(),
			repo: comp.repo.to_owned(),
			number: comp.number,
		})),
		other => other,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const COMPANION_MARKERS: &[&str; 2] = &["Companion", "companion"];

	#[test]
	fn test_companion_parsing_url_params() {
		for companion_marker in COMPANION_MARKERS {
			// Extra params should not be included in the parsed URL
			assert_eq!(
				parse_companion_from_url(&format!(
					"{}: https://github.com/org/repo/pull/1234?extra_params=true",
					companion_marker
				)),
				Some(PullRequestDetailsWithHtmlUrl {
					html_url: "https://github.com/org/repo/pull/1234"
						.to_owned(),
					owner: "org".to_owned(),
					repo: "repo".to_owned(),
					number: 1234
				})
			);
		}
	}

	#[test]
	fn test_companion_parsing_all_markers() {
		for companion_marker in COMPANION_MARKERS {
			// Long version should work even if the body has some other content around
			// the companion text
			assert_eq!(
				parse_companion_from_url(&format!(
					"
					Companion line is in the middle
					{}: https://github.com/org/repo/pull/1234
					Final line
					",
					companion_marker
				)),
				Some(PullRequestDetailsWithHtmlUrl {
					html_url: "https://github.com/org/repo/pull/1234"
						.to_owned(),
					owner: "org".to_owned(),
					repo: "repo".to_owned(),
					number: 1234
				})
			);
		}
	}

	#[test]
	fn test_companion_parsing_short_version_wrap() {
		for companion_marker in COMPANION_MARKERS {
			// Short version should work even if the body has some other content around
			// the companion text
			assert_eq!(
				parse_companion_from_url(&format!(
					"
					Companion line is in the middle
					{}: org/repo#1234
					Final line
					",
					companion_marker
				)),
				Some(PullRequestDetailsWithHtmlUrl {
					html_url: "https://github.com/org/repo/pull/1234"
						.to_owned(),
					owner: "org".to_owned(),
					repo: "repo".to_owned(),
					number: 1234
				})
			);
		}
	}

	#[test]
	fn test_companion_parsing_long_version_same_line() {
		for companion_marker in COMPANION_MARKERS {
			// Long version should not be detected if "companion: " and the expression
			// are not both in the same line
			assert_eq!(
				parse_companion_from_url(&format!(
					"
					I want to talk about {}: but NOT reference it
					I submitted it in https://github.com/org/repo/pull/1234
					",
					companion_marker
				)),
				None
			);
		}
	}

	#[test]
	fn test_companion_parsing_short_version_same_line() {
		for companion_marker in COMPANION_MARKERS {
			// Short version should not be detected if "companion: " and the expression are not both in
			// the same line
			assert_eq!(
				parse_companion_from_url(&format!(
					"
					I want to talk about {}: but NOT reference it
					I submitted it in org/repo#1234
					",
					companion_marker
				)),
				None
			);
		}
	}

	#[test]
	fn test_companion_parsing_multiple_companions() {
		let owner = "org";
		let repo = "repo";
		let pr_number = 1234;
		let companion_url =
			format!("https://github.com/{}/{}/pull/{}", owner, repo, pr_number);
		let expected_companion = PullRequestDetailsWithHtmlUrl {
			html_url: companion_url.to_owned(),
			owner: owner.into(),
			repo: repo.into(),
			number: pr_number,
		};
		for companion_marker in COMPANION_MARKERS {
			assert_eq!(
				parse_all_companions(
					&[],
					&format!(
						"
						first {}: {}
						second {}: {}
					",
						companion_marker,
						&companion_url,
						companion_marker,
						&companion_url
					)
				),
				vec![expected_companion.clone(), expected_companion.clone()]
			);
		}
	}

	#[test]
	fn test_cyclical_references() {
		let owner = "org";
		let repo = "repo";

		for companion_marker in COMPANION_MARKERS {
			let companion_description = format!(
				"
				{}: https://github.com/{}/{}/pull/123
				",
				companion_marker, owner, repo,
			);

			// If the source is not referenced in the description, something is parsed
			assert_ne!(
				parse_all_companions(&[], &companion_description),
				vec![]
			);

			// If the source is referenced in the description, it is omitted
			assert_eq!(
				parse_all_companions(
					&[CompanionReferenceTrailItem {
						owner: owner.into(),
						repo: repo.into()
					}],
					&companion_description
				),
				vec![]
			);
		}
	}

	#[test]
	fn test_restricted_regex() {
		let owner = "org";
		let repo = "repo";
		let pr_number = 1234;
		let companion_url = format!("{}/{}#{}", owner, repo, pr_number);
		for companion_marker in COMPANION_MARKERS {
			assert_eq!(
				parse_all_companions(
					&[],
					// the companion expression should not be matched because of the " for" part
					&format!("{} for {}", companion_marker, &companion_url)
				),
				vec![]
			);
		}
	}
}
