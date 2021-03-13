use httptest::{matchers::*, responders::*, Expectation, Server};
use parity_processbot::{github, setup::setup, webhook::handle_payload};
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

	let state = setup().await.unwrap();
	let irrelevant_url = "https://foo.bar".to_string();
	let irrelevant_user = github::User {
		login: "foo".to_string(),
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
				body: Some("foo".to_string()),
				html_url: irrelevant_url.clone(),
				repository_url: Some(irrelevant_url.clone()),
				pull_request: Some(github::IssuePullRequest {}),
			},
		},
		&state,
	)
	.await
	.unwrap();
}
