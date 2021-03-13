use httptest::{matchers::*, responders::*, Expectation, Server};
use parity_processbot::{
	config::MainConfig, github, github_bot, setup::setup,
	webhook::handle_payload,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn case1() {
	env_logger::init();

	let placeholder_string = "".to_string();
	let placeholder_user = github::User {
		login: "foo".to_string(),
	};

	let github_api = Server::run();
	github::BASE_API_URL
		.set(github_api.url("").to_string())
		.unwrap();

	let substrate_pr_number = 1;
	let substrate_repository_url = "https://github.com/_/substrate";
	let substrate_pr_url =
		format!("{}/pull/{}", substrate_repository_url, substrate_pr_number);

	let substrate_api_merge_path = format!(
		"/{}/repos/{}/{}/pulls/{}/merge",
		github::base_api_url(),
		"_",
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

	let companion_api_merge_path = format!(
		"/{}/repos/{}/{}/pulls/{}/merge",
		github::base_api_url(),
		"_",
		"polkadot",
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
		Some(github_bot::GithubBot::new_for_testing(Some(
			"/todo/fs/git".to_string(),
		))),
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
}
