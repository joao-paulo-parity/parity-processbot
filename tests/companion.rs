use httptest::{matchers::*, responders::*, Expectation, Server};
use parity_processbot::{
	config::MainConfig, github, github_bot, setup::setup,
	webhook::handle_payload,
};
use std::fs;
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

	let git_daemon_dir = tempfile::tempdir().unwrap();
	let git_daemon_port = utils::get_available_port().unwrap();
	let git_fetch_url = format!("git://127.0.0.1:{}", git_daemon_port);

	let substrate_repo_dir =
		git_daemon_dir.path().join("substrate").join("substrate");
	fs::create_dir_all(&substrate_repo_dir).unwrap();
	Command::new("git")
		.arg("init")
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
	fs::create_dir(substrate_src_dir).unwrap();
	fs::write(substrate_src_dir.join("main.rs"), "fn main() {}").unwrap();
	Command::new("git")
		.arg("add")
		.arg(".")
		.current_dir(&substrate_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();
	Command::new("git")
		.arg("commit")
		.arg("-m")
		.arg("init")
		.current_dir(&substrate_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();

	let companion_repo_dir =
		git_daemon_dir.path().join("companion").join("companion");
	fs::create_dir_all(&companion_repo_dir).unwrap();
	Command::new("git")
		.arg("init")
		.current_dir(&substrate_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();
	fs::write(
		&substrate_repo_dir.join("Cargo.toml"),
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
		.current_dir(&substrate_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();
	Command::new("git")
		.arg("commit")
		.arg("-m")
		.arg("init")
		.current_dir(&substrate_repo_dir)
		.spawn()
		.unwrap()
		.await
		.unwrap();

	let _ = Command::new("git")
		.arg("daemon")
		.arg(format!("--port={}", git_daemon_port))
		.arg("--base-path=.")
		.arg("--export-all")
		.current_dir(git_daemon_dir.path())
		.spawn()
		.unwrap();

	let github_api = Server::run();
	github::BASE_API_URL
		.set(github_api.url("").to_string())
		.unwrap();

	let substrate_pr_number = 1;
	let substrate_repository_url = "https://github.com/substrate/substrate";
	let substrate_pr_url =
		format!("{}/pull/{}", substrate_repository_url, substrate_pr_number);
	let substrate_api_merge_path = format!(
		"/{}/repos/{}/{}/pulls/{}/merge",
		github::base_api_url(),
		"substrate",
		"substrate",
		1
	);
	github_api.expect(
		Expectation::matching(request::method_path(
			"PUT",
			substrate_api_merge_path,
		))
		.respond_with(status_code(200)),
	);

	let companion_pr_number = 1;
	let companion_repository_url = "https://github.com/companion/companion";
	let companion_pr_url =
		format!("{}/pull/{}", companion_repository_url, companion_pr_number);
	let companion_api_merge_path = format!(
		"/{}/repos/{}/{}/pulls/{}/merge",
		github::base_api_url(),
		"companion",
		"companion",
		1
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

	let state = setup(
		Some(MainConfig {
			environment: placeholder_string.clone(),
			test_repo: placeholder_string.clone(),
			installation_login: placeholder_string.clone(),
			webhook_secret: placeholder_string.clone(),
			webhook_port: placeholder_string.clone(),
			db_path: placeholder_string.clone(),
			bamboo_token: placeholder_string.clone(),
			private_key: vec![],
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
		Some(github_bot::GithubBot::new_for_testing(Some(git_fetch_url))),
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
				id: 1,
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
				id: 1,
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
