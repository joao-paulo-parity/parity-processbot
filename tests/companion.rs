use httptest::{matchers::*, responders::*, Expectation, Server};
use parity_processbot::{
	config::MainConfig, github, setup::setup, webhook::handle_payload,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn case1() {
	env_logger::init();

	let github_api = Server::run();
	github::BASE_URL
		.set(Some(github_api.url("").to_string()))
		.unwrap();

	let substrate_merge_path = format!(
		"/{}/repos/{}/{}/pulls/{}/merge",
		github::base_url(),
		"_",
		"substrate",
		1
	);
	let companion_merge_path = format!(
		"/{}/repos/{}/{}/pulls/{}/merge",
		github::base_url(),
		"_",
		"polkadot",
		1
	);

	github_api.expect(
		Expectation::matching(request::method_path(
			"PUT",
			substrate_merge_path,
		))
		.respond_with(status_code(200)),
	);

	let companion_merge_tries = Arc::new(AtomicUsize::new(0));
	github_api.expect(
		Expectation::matching(request::method_path(
			"PUT",
			companion_merge_path,
		))
		.respond_with(move || {
			if companion_merge_tries.fetch_add(1, Ordering::SeqCst) == 1 {
				status_code(405)
			} else {
				status_code(200)
			}
		}),
	);

	let irrelevant_string = "".to_string();

	let state = setup(Some(MainConfig {
		environment: irrelevant_string.clone(),
		test_repo: irrelevant_string.clone(),
		installation_login: irrelevant_string.clone(),
		webhook_secret: irrelevant_string.clone(),
		webhook_port: irrelevant_string.clone(),
		db_path: irrelevant_string.clone(),
		bamboo_token: irrelevant_string.clone(),
		private_key: vec![],
		matrix_homeserver: irrelevant_string.clone(),
		matrix_access_token: irrelevant_string.clone(),
		matrix_default_channel_id: irrelevant_string.clone(),
		main_tick_secs: 0,
		bamboo_tick_secs: 0,
		matrix_silent: true,
		gitlab_hostname: irrelevant_string.clone(),
		gitlab_project: irrelevant_string.clone(),
		gitlab_job_name: irrelevant_string.clone(),
		gitlab_private_token: irrelevant_string.clone(),
	}))
	.await
	.unwrap();

	let irrelevant_user = github::User {
		login: irrelevant_string.clone(),
	};

	handle_payload(
		github::Payload::IssueComment {
			action: github::IssueCommentAction::Created,
			comment: github::Comment {
				body: "bot merge".to_string(),
				user: irrelevant_user.clone(),
			},
			issue: github::Issue {
				id: 1,
				number: 1,
				body: Some(irrelevant_string.clone()),
				html_url: irrelevant_string.clone(),
				repository_url: Some(irrelevant_string.clone()),
				pull_request: Some(github::IssuePullRequest {}),
			},
		},
		&state,
	)
	.await
	.unwrap();
}
