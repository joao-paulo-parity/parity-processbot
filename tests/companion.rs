use httptest::{matchers::*, responders::*, Expectation, Server};
use parity_processbot::{
	config::{BotConfig, MainConfig},
	github, github_bot, gitlab_bot, matrix_bot,
	setup::setup,
	webhook::handle_payload,
};
use std::fs;
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::process::Command;

mod utils;

#[tokio::test]
async fn case1() {
	env_logger::init();

	let placeholder_string = "".to_string();
	let placeholder_user = github::User {
		login: "foo".to_string(),
	};
	let placeholder_number = 1;
	let placeholder_sha = "MDEwOlJlcG9zaXRvcnkxMDk4NzI2MjA=";

	let db_dir = tempfile::tempdir().unwrap();

	let git_daemon_dir = tempfile::tempdir().unwrap();
	let git_daemon_port = utils::get_available_port().unwrap();
	let git_fetch_url = format!("git://127.0.0.1:{}", git_daemon_port);

	let substrate_org = "substrate";
	let substrate_repo = "substrate";
	let substrate_repo_dir = git_daemon_dir
		.path()
		.join(substrate_org)
		.join(substrate_repo);
	fs::create_dir_all(&substrate_repo_dir).unwrap();
	Command::new("git")
		.arg("init")
		.stdout(Stdio::null())
		.current_dir(&substrate_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();
	fs::write(
		&substrate_repo_dir.join("Cargo.toml"),
		r#"
[package]
name = "substrate"
version = "0.0.1"
authors = ["substrate <substrate@substrate.com>"]
description = "substrate"
"#,
	)
	.unwrap();

	let substrate_src_dir = &substrate_repo_dir.join("src");
	fs::create_dir(&substrate_src_dir).unwrap();
	fs::write((&substrate_src_dir).join("main.rs"), "fn main() {}").unwrap();

	Command::new("git")
		.arg("add")
		.arg(".")
		.current_dir(&substrate_repo_dir)
		.stdout(Stdio::null())
		.spawn()
		.unwrap()
		.await
		.unwrap();
	Command::new("git")
		.arg("commit")
		.arg("-m")
		.arg("init")
		.stdout(Stdio::null())
		.current_dir(&substrate_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();
	let substrate_head_sha_cmd = Command::new("git")
		.arg("rev-parse")
		.arg("HEAD")
		.current_dir(&substrate_repo_dir)
		.output()
		.await
		.unwrap();
	let substrate_head_sha = String::from_utf8(substrate_head_sha_cmd.stdout)
		.unwrap()
		.trim()
		.to_string();

	let companion_org = "companion";
	let companion_repo = "companion";
	let companion_repo_dir = git_daemon_dir
		.path()
		.join(companion_org)
		.join(companion_repo);
	fs::create_dir_all(&companion_repo_dir).unwrap();
	Command::new("git")
		.arg("init")
		.stdout(Stdio::null())
		.current_dir(&companion_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();
	fs::write(
		&companion_repo_dir.join("Cargo.toml"),
		r#"
[package]
name = "companion"
version = "0.0.1"
authors = ["companion <companion@companion.com>"]
description = "companion"

[dependencies]
"#
		.to_string() + format!(
"substrate = {{ git = \"{}/substrate/substrate\", branch = \"master\" }}",
git_fetch_url
)
		.as_str(),
	)
	.unwrap();

	let companion_src_dir = &companion_repo_dir.join("src");
	fs::create_dir(&companion_src_dir).unwrap();
	fs::write((&companion_src_dir).join("main.rs"), "fn main() {}").unwrap();

	Command::new("git")
		.arg("add")
		.arg(".")
		.stdout(Stdio::null())
		.current_dir(&companion_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();
	Command::new("git")
		.arg("commit")
		.arg("-m")
		.arg("init")
		.stdout(Stdio::null())
		.current_dir(&companion_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();

	// Hold onto the git daemon process handle until the test is done
	let _ = Command::new("git")
		.arg("daemon")
		.arg(format!("--port={}", git_daemon_port))
		.arg("--base-path=.")
		.arg("--export-all")
		.stdout(Stdio::null())
		.current_dir(git_daemon_dir.path())
		.spawn()
		.unwrap();

	let github_api = Server::run();
	github::BASE_API_URL
		.set(github_api.url("").to_string())
		.unwrap();

	let substrate_pr_number = 1;
	let substrate_repository_url =
		format!("https://foo.com/{}/{}", substrate_org, substrate_repo);
	let substrate_pr_url =
		format!("{}/pull/{}", substrate_repository_url, substrate_pr_number);
	github_api.expect(
		Expectation::matching(request::method_path(
			"PUT",
			format!(
				"/{}/repos/{}/{}/pulls/{}/merge",
				github::base_api_url(),
				substrate_org,
				substrate_repo,
				substrate_pr_number
			),
		))
		.respond_with(status_code(200)),
	);
	github_api.expect(
		Expectation::matching(request::method_path(
			"PUT",
			format!(
				"/{}/repos/{}/{}/pulls/{}",
				github::base_api_url(),
				substrate_org,
				substrate_repo,
				substrate_pr_number
			),
		))
		.respond_with(json_encoded(github::PullRequest {
			body: Some(placeholder_string.clone()),
			number: substrate_pr_number,
			labels: vec![],
			mergeable: Some(true),
			html_url: substrate_pr_url.clone(),
			url: substrate_pr_url.clone(),
			user: Some(placeholder_user.clone()),
			base: github::Base {
				ref_field: "master".to_string(),
				sha: substrate_head_sha,
				repo: github::HeadRepo {
					name: substrate_repo.to_string(),
					owner: Some(github::User {
						login: substrate_org.to_string(),
					}),
				},
			},
			head: github::Head {
				ref_field: "develop".to_string(),
				sha: placeholder_sha.to_string(),
				repo: github::HeadRepo {
					name: substrate_repo.to_string(),
					owner: Some(github::User {
						login: substrate_org.to_string(),
					}),
				},
			},
		})),
	);

	let companion_pr_number = 1;
	let companion_repository_url = "https://github.com/companion/companion";
	let companion_pr_url =
		format!("{}/pull/{}", companion_repository_url, companion_pr_number);
	let companion_api_merge_path = format!(
		"/{}/repos/{}/{}/pulls/{}/merge",
		github::base_api_url(),
		companion_org,
		companion_repo,
		companion_pr_number
	);
	let companion_merge_tries = Arc::new(AtomicUsize::new(0));
	github_api.expect(
		Expectation::matching(request::method_path(
			"PUT",
			companion_api_merge_path,
		))
		.respond_with(move || {
			if companion_merge_tries.fetch_add(1, Ordering::SeqCst) == 1 {
				status_code(405)
			} else {
				status_code(200)
			}
		}),
	);

	let placeholder_private_key = "-----BEGIN RSA PRIVATE KEY-----
MIIBPQIBAAJBAOsfi5AGYhdRs/x6q5H7kScxA0Kzzqe6WI6gf6+tc6IvKQJo5rQc
dWWSQ0nRGt2hOPDO+35NKhQEjBQxPh/v7n0CAwEAAQJBAOGaBAyuw0ICyENy5NsO
2gkT00AWTSzM9Zns0HedY31yEabkuFvrMCHjscEF7u3Y6PB7An3IzooBHchsFDei
AAECIQD/JahddzR5K3A6rzTidmAf1PBtqi7296EnWv8WvpfAAQIhAOvowIXZI4Un
DXjgZ9ekuUjZN+GUQRAVlkEEohGLVy59AiEA90VtqDdQuWWpvJX0cM08V10tLXrT
TTGsEtITid1ogAECIQDAaFl90ZgS5cMrL3wCeatVKzVUmuJmB/VAmlLFFGzK0QIh
ANJGc7AFk4fyFD/OezhwGHbWmo/S+bfeAiIh2Ss2FxKJ
-----END RSA PRIVATE KEY-----"
		.as_bytes()
		.to_vec();

	let state = setup(
		Some(MainConfig {
			environment: placeholder_string.clone(),
			test_repo: placeholder_string.clone(),
			installation_login: placeholder_string.clone(),
			webhook_secret: placeholder_string.clone(),
			webhook_port: placeholder_string.clone(),
			db_path: (&db_dir).path().display().to_string(),
			bamboo_token: placeholder_string.clone(),
			private_key: placeholder_private_key.clone(),
			matrix_homeserver: placeholder_string.clone(),
			matrix_access_token: placeholder_string.clone(),
			matrix_default_channel_id: placeholder_string.clone(),
			main_tick_secs: 0,
			bamboo_tick_secs: 0,
			matrix_silent: true,
			gitlab_hostname: placeholder_string.clone(),
			gitlab_project: placeholder_string.clone(),
			gitlab_job_name: placeholder_string.clone(),
			gitlab_private_token: placeholder_string.clone(),
		}),
		Some(BotConfig {
			status_failure_ping: 0,
			issue_not_addressed_ping: 0,
			issue_not_assigned_to_pr_author_ping: 0,
			no_project_author_is_core_ping: 0,
			no_project_author_is_core_close_pr: 0,
			no_project_author_unknown_close_pr: 0,
			project_confirmation_timeout: 0,
			review_request_ping: 0,
			private_review_reminder_ping: 0,
			public_review_reminder_ping: 0,
			public_review_reminder_delay: 0,
			min_reviewers: 0,
			core_sorting_repo_name: placeholder_string.clone(),
			logs_room_id: placeholder_string.clone(),
		}),
		Some(matrix_bot::MatrixBot::new_placeholder_for_testing()),
		Some(gitlab_bot::GitlabBot::new_placeholder_for_testing()),
		Some(github_bot::GithubBot::new_for_testing(
			placeholder_private_key.clone(),
			git_fetch_url,
		)),
		false,
	)
	.await
	.unwrap();

	handle_payload(
		github::Payload::IssueComment {
			action: github::IssueCommentAction::Created,
			comment: github::Comment {
				body: "bot merge".to_string(),
				user: placeholder_user.clone(),
			},
			issue: github::Issue {
				id: placeholder_number,
				number: substrate_pr_number,
				body: Some(placeholder_string.clone()),
				html_url: substrate_pr_url,
				repository_url: Some(substrate_repository_url.to_string()),
				pull_request: Some(github::IssuePullRequest {}),
			},
		},
		&state,
	)
	.await
	.unwrap();

	handle_payload(
		github::Payload::IssueComment {
			action: github::IssueCommentAction::Created,
			comment: github::Comment {
				body: "bot merge".to_string(),
				user: placeholder_user.clone(),
			},
			issue: github::Issue {
				id: placeholder_number,
				number: companion_pr_number,
				body: Some(placeholder_string.clone()),
				html_url: companion_pr_url,
				repository_url: Some(companion_repository_url.to_string()),
				pull_request: Some(github::IssuePullRequest {}),
			},
		},
		&state,
	)
	.await
	.unwrap();
}
