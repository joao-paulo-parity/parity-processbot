use regex::Regex;
use snafu::ResultExt;
use std::path::Path;

use crate::{cmd::*, error::*, github_bot::GithubBot, Result};

pub async fn companion_update(
	github_bot: &GithubBot,
	owner: &str,
	owner_repo: &str,
	contributor: &str,
	contributor_repo: &str,
	contributor_branch: &str,
) -> Result<String> {
	let token = github_bot.client.auth_key().await?;

	let owner_repository_domain =
		format!("github.com/{}/{}.git", owner, owner_repo);
	let owner_remote_address = format!(
		"https://x-access-token:{}@{}",
		token, owner_repository_domain
	);
	let repo_dir = format!("./{}", owner_repo);

	if Path::new(&repo_dir).exists() {
		log::info!("{} is already cloned; skipping", owner_repository_domain);
	} else {
		run_cmd_in_cwd(
			"git",
			&["clone", "-v", &owner_remote_address],
			// Can't be logged directly because the access token is included in the remote address
			CommandMessage::SubstituteFor(CommandMessages {
				cmd_display: Some(format!(
					"git clone https://x-access-token:${{SECRET}}@{}",
					owner_repository_domain,
				)),
				on_failure: Some(format!(
					"Failed to clone {}",
					owner_repository_domain
				)),
			}),
		)
		.await?;
	}

	let contributor_repository_domain =
		format!("github.com/{}/{}.git", contributor, contributor_repo);
	let contributor_remote_branch =
		format!("{}/{}", contributor, contributor_branch);
	let contributor_remote_address = format!(
		"https://x-access-token:{}@{}",
		token, contributor_repository_domain
	);

	// The contributor's remote entry might exist from a previous run (not expected for a fresh
	// clone). If so, delete it so that it can be recreated.
	if run_cmd(
		"git",
		&["remote", "get-url", contributor],
		&repo_dir,
		CommandMessage::EnabledWithErrorsSilenced,
	)
	.await
	.is_ok()
	{
		run_cmd(
			"git",
			&["remote", "remove", contributor],
			&repo_dir,
			CommandMessage::Enabled,
		)
		.await?;
	}
	run_cmd(
		"git",
		&["remote", "add", contributor, &contributor_remote_address],
		&repo_dir,
		// Can't be logged directly because the access token is included in the remote address
		CommandMessage::SubstituteFor(CommandMessages {
			cmd_display: Some(format!(
				"git remote add {} https://x-access-token:${{SECRET}}@{}",
				contributor, contributor_remote_address,
			)),
			on_failure: Some(format!(
				"Failed to add remote for {}",
				contributor_repository_domain
			)),
		}),
	)
	.await?;

	run_cmd(
		"git",
		&["fetch", contributor, contributor_branch],
		&repo_dir,
		CommandMessage::Enabled,
	)
	.await?;

	// The contributor's branch might exist from a previous run (not expected for a fresh clone).
	// If so, delete it so that it can be recreated.
	if run_cmd(
		"git",
		&["show-branch", contributor_branch],
		&repo_dir,
		CommandMessage::EnabledWithErrorsSilenced,
	)
	.await
	.is_ok()
	{
		run_cmd(
			"git",
			&["branch", "-D", contributor_branch],
			&repo_dir,
			CommandMessage::Enabled,
		)
		.await?;
	}
	run_cmd(
		"git",
		&["checkout", "--track", &contributor_remote_branch],
		&repo_dir,
		CommandMessage::Enabled,
	)
	.await?;

	let owner_remote = "origin";
	// FIXME !! CHANGE THIS BEFORE DEPLOYING TO PRODUCTION
	let owner_branch = "test";
	let owner_remote_branch = format!("{}/{}", owner_remote, owner_branch);

	run_cmd(
		"git",
		&["fetch", owner_remote, owner_branch],
		&repo_dir,
		CommandMessage::Enabled,
	)
	.await?;

	// Create master merge commit before updating packages
	let master_merge_result = run_cmd(
		"git",
		&["merge", &owner_remote_branch, "--no-ff", "--no-edit"],
		&repo_dir,
		CommandMessage::Enabled,
	)
	.await;
	if let Err(e) = master_merge_result {
		log::info!("Aborting companion update due to master merge failure");
		run_cmd(
			"git",
			&["merge", "--abort"],
			&repo_dir,
			CommandMessage::Enabled,
		)
		.await?;
		return Err(e);
	}

	// `cargo update` should normally make changes to the lockfile with the latest SHAs from Github
	run_cmd(
		"cargo",
		&["update", "-vp", "sp-io"],
		&repo_dir,
		CommandMessage::Enabled,
	)
	.await?;

	// Check if `cargo update` resulted in any changes. If the master merge commit already had the
	// latest lockfile then no changes might have been made.
	let changes_after_update_output = run_cmd_with_output(
		"git",
		&["status", "--short"],
		&repo_dir,
		CommandMessage::Enabled,
	)
	.await?;
	if !String::from_utf8_lossy(&(&changes_after_update_output).stdout[..])
		.trim()
		.is_empty()
	{
		run_cmd(
			"git",
			&["commit", "-am", "update Substrate"],
			&repo_dir,
			CommandMessage::Enabled,
		)
		.await?;
	}

	run_cmd(
		"git",
		&["push", contributor, contributor_branch],
		&repo_dir,
		CommandMessage::Enabled,
	)
	.await?;

	log::info!(
		"Getting the head SHA after a companion update in {}",
		&contributor_remote_branch
	);
	let updated_sha_output = run_cmd_with_output(
		"git",
		&["rev-parse", "HEAD"],
		&repo_dir,
		CommandMessage::Enabled,
	)
	.await?;
	let updated_sha = String::from_utf8(updated_sha_output.stdout)
		.context(Utf8)?
		.trim()
		.to_string();

	Ok(updated_sha)
}

pub fn companion_parse(body: &str) -> Option<(String, String, String, i64)> {
	companion_parse_long(body).or(companion_parse_short(body))
}

fn companion_parse_long(body: &str) -> Option<(String, String, String, i64)> {
	let re = Regex::new(
		r"companion.*(?P<html_url>github.com/(?P<owner>[^/]+)/(?P<repo>[^/]+)/pull/(?P<number>[[:digit:]]+))"
	)
	.unwrap();
	let caps = re.captures(&body)?;
	let html_url = caps.name("html_url")?.as_str().to_owned();
	let owner = caps.name("owner")?.as_str().to_owned();
	let repo = caps.name("repo")?.as_str().to_owned();
	let number = caps
		.name("number")?
		.as_str()
		.to_owned()
		.parse::<i64>()
		.ok()?;
	Some((html_url, owner, repo, number))
}

fn companion_parse_short(body: &str) -> Option<(String, String, String, i64)> {
	let re = Regex::new(
		r"companion.*(?P<owner>[^/]+)/(?P<repo>[^/]+)#(?P<number>[[:digit:]]+)",
	)
	.unwrap();
	let caps = re.captures(&body)?;
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
	Some((html_url, owner, repo, number))
}

#[cfg(test)]
mod tests {
	//use super::*;

	//#[test]
	//fn test_companion_parse() {
	//assert_eq!(
	//companion_parse(
	//"companion: https://github.com/paritytech/polkadot/pull/1234"
	//),
	//Some((
	//"https://github.com/paritytech/polkadot/pull/1234".to_owned(),
	//"paritytech".to_owned(),
	//"polkadot".to_owned(),
	//1234
	//))
	//);
	//assert_eq!(
	//companion_parse(
	//"\nthis is a companion pr https://github.com/paritytech/polkadot/pull/1234"
	//),
	//Some((
	//"https://github.com/paritytech/polkadot/pull/1234".to_owned(),
	//"paritytech".to_owned(),
	//"polkadot".to_owned(),
	//1234
	//))
	//);
	//assert_eq!(
	//companion_parse(
	//"\nthis is some other pr https://github.com/paritytech/polkadot/pull/1234"
	//),
	//None,
	//);
	//assert_eq!(
	//companion_parse(
	//"\nthis is a companion pr https://github.com/paritytech/polkadot/pull/1234/plus+some&other_stuff"
	//),
	//Some((
	//"https://github.com/paritytech/polkadot/pull/1234".to_owned(),
	//"paritytech".to_owned(),
	//"polkadot".to_owned(),
	//1234
	//))
	//);
	//assert_eq!(
	//companion_parse("companion\nparitytech/polkadot#1234"),
	//None
	//);
	//assert_eq!(
	//companion_parse("companion: paritytech/polkadot#1234"),
	//Some((
	//"https://github.com/paritytech/polkadot/pull/1234".to_owned(),
	//"paritytech".to_owned(),
	//"polkadot".to_owned(),
	//1234
	//))
	//);
	//assert_eq!(
	//companion_parse("companion: paritytech/polkadot/1234"),
	//None
	//);
	//assert_eq!(
	//companion_parse("stuff\ncompanion pr: paritytech/polkadot#1234"),
	//Some((
	//"https://github.com/paritytech/polkadot/pull/1234".to_owned(),
	//"paritytech".to_owned(),
	//"polkadot".to_owned(),
	//1234
	//))
	//);
	//}
}
