use httptest::{matchers::*, responders::*, Expectation, Server};
use insta::assert_snapshot;
use parity_processbot::{
	config::{BotConfig, MainConfig},
	constants::*,
	github, github_bot, gitlab_bot, matrix_bot,
	setup::setup,
	webhook::handle_payload,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;

mod utils;

use utils::*;

//#[tokio::test]
//async fn team_lead_merges_pr() {
//let log_dir = tempfile::tempdir().unwrap();
//flexi_logger::Logger::with_env_or_str("parity_processbot=info")
//.log_to_file()
//.directory((&log_dir).path().to_path_buf())
//.duplicate_to_stdout(flexi_logger::Duplicate::All)
//.start()
//.unwrap();

//let bot_username = "bot";
//let placeholder_user = github::User {
//login: "foo".to_string(),
//type_field: Some(github::UserType::User),
//};
//let placeholder_sha = "MDEwOlJlcG9zaXRvcnkxMDk4NzI2MjA";
//let placeholder_id: usize = 1;

//let db_dir = tempfile::tempdir().unwrap();

//let git_daemon_dir = tempfile::tempdir().unwrap();
//let git_daemon_port = get_available_port().unwrap();
//let git_fetch_url = &format!("git://127.0.0.1:{}", git_daemon_port);

//let org = "org";
//let teamleads_team_id = 1;
//let coredevs_team_id = 2;
//let substrate_repo_name = "substrate";
//let substrate_repo_dir =
//git_daemon_dir.path().join(org).join(substrate_repo_name);
//let substrate_user = &github::User {
//login: org.to_string(),
//type_field: Some(github::UserType::User),
//};
//let substrate_repo = github::Repository {
//name: substrate_repo_name.to_string(),
//full_name: Some(format!("{}/{}", org, substrate_repo_name)),
//owner: Some(substrate_user.clone()),
//html_url: "".to_string(),
//};
//fs::create_dir_all(&substrate_repo_dir).unwrap();
//Command::new("git")
//.arg("init")
//.arg("-b")
//.arg("master")
//.stdout(Stdio::null())
//.current_dir(&substrate_repo_dir)
//.spawn()
//.unwrap()
//.await
//.unwrap();
//fs::write(
//&substrate_repo_dir.join("Cargo.toml"),
//r#"
//[package]
//name = "substrate"
//version = "0.0.1"
//authors = ["substrate <substrate@substrate.com>"]
//description = "substrate"
//"#,
//)
//.unwrap();

//let substrate_src_dir = &substrate_repo_dir.join("src");
//fs::create_dir(&substrate_src_dir).unwrap();
//fs::write((&substrate_src_dir).join("main.rs"), "fn main() {}").unwrap();

//Command::new("git")
//.arg("add")
//.arg(".")
//.current_dir(&substrate_repo_dir)
//.stdout(Stdio::null())
//.spawn()
//.unwrap()
//.await
//.unwrap();
//Command::new("git")
//.arg("commit")
//.arg("-m")
//.arg("init")
//.stdout(Stdio::null())
//.current_dir(&substrate_repo_dir)
//.spawn()
//.unwrap()
//.await
//.unwrap();
//let substrate_head_sha_cmd = Command::new("git")
//.arg("rev-parse")
//.arg("HEAD")
//.current_dir(&substrate_repo_dir)
//.output()
//.await
//.unwrap();
//let substrate_head_sha = String::from_utf8(substrate_head_sha_cmd.stdout)
//.unwrap()
//.trim()
//.to_string();

//// Hold onto the git daemon process handle until the test is done
//let _ = Command::new("git")
//.arg("daemon")
//.arg(format!("--port={}", git_daemon_port))
//.arg("--base-path=.")
//.arg("--export-all")
//.kill_on_drop(true)
//.stdout(Stdio::null())
//.current_dir(git_daemon_dir.path())
//.spawn()
//.unwrap();

//let github_api = Server::run();
//let api_base_url = &github_api.url("").to_string();
//let api_base_url = &api_base_url[0..api_base_url.len() - 1];
//github::BASE_API_URL.set(api_base_url.to_owned()).unwrap();
//github_api.expect(
//Expectation::matching(request::method_path(
//"GET",
//"/app/installations",
//))
//.respond_with(json_encoded(vec![github::Installation {
//id: 1,
//account: github::User {
//login: bot_username.to_string(),
//type_field: Some(github::UserType::Bot),
//},
//}])),
//);
//github_api.expect(
//Expectation::matching(request::method_path(
//"POST",
//format!("/app/installations/{}/access_tokens", 1),
//))
//.respond_with(json_encoded(github::InstallationToken {
//token: "DOES_NOT_MATTER".to_string(),
//expires_at: None,
//})),
//);

//for (team_name, team_id) in &[
//(CORE_DEVS_GROUP, coredevs_team_id),
//(SUBSTRATE_TEAM_LEADS_GROUP, teamleads_team_id),
//] {
//github_api.expect(
//Expectation::matching(request::method_path(
//"GET",
//format!("/orgs/{}/teams/{}", org, team_name),
//))
//.times(0..)
//.respond_with(json_encoded(github::Team { id: *team_id })),
//);
//github_api.expect(
//Expectation::matching(request::method_path(
//"GET",
//format!("/teams/{}/members", team_id),
//))
//.times(0..)
//.respond_with(json_encoded(vec![placeholder_user.clone()])),
//);
//}

//let substrate_pr_number = 1;
//let substrate_repository_url =
//format!("https://github.com/{}/{}", org, substrate_repo_name);
//let substrate_pr_url =
//format!("{}/pull/{}", substrate_repository_url, substrate_pr_number);
//let substrate_pr_api_path = &format!(
//"/repos/{}/{}/pull/{}",
//org, substrate_repo_name, substrate_pr_number
//);
//let substrate_pr_issue_api_path = &format!(
//"/repos/{}/{}/issues/{}",
//org, substrate_repo_name, substrate_pr_number
//);
//let substrate_pr_api_url =
//&format!("{}/{}", api_base_url, substrate_pr_api_path);
//github_api.expect(
//Expectation::matching(request::method_path(
//"PUT",
//format!(
//"/repos/{}/{}/pulls/{}/merge",
//org, substrate_repo_name, substrate_pr_number
//),
//))
//.respond_with(status_code(200)),
//);
//github_api.expect(
//Expectation::matching(request::method_path(
//"GET",
//format!("/orgs/{}/members/{}", org, (&placeholder_user).login),
//))
//.respond_with(
//status_code(204)
//.append_header("Content-Type", "application/json")
//.body(serde_json::to_string(&json!({})).unwrap()),
//),
//);
//github_api.expect(
//Expectation::matching(request::method_path(
//"GET",
//format!(
//"/repos/{}/{}/pulls/{}",
//org, substrate_repo_name, substrate_pr_number
//),
//))
//.respond_with(json_encoded(github::PullRequest {
//body: Some("".to_string()),
//number: substrate_pr_number,
//labels: vec![],
//mergeable: Some(true),
//html_url: substrate_pr_url.clone(),
//url: substrate_pr_api_url.clone(),
//user: Some(placeholder_user.clone()),
//repository: Some(substrate_repo.clone()),
//base: github::Base {
//ref_field: Some("master".to_string()),
//sha: Some(substrate_head_sha),
//repo: Some(github::HeadRepo {
//name: substrate_repo_name.to_string(),
//owner: Some(substrate_user.clone()),
//}),
//},
//head: Some(github::Head {
//ref_field: Some("develop".to_string()),
//sha: Some(placeholder_sha.to_string()),
//repo: Some(github::HeadRepo {
//name: substrate_repo_name.to_string(),
//owner: Some(substrate_user.clone()),
//}),
//}),
//})),
//);
//github_api.expect(
//Expectation::matching(request::method_path(
//"GET",
//format!("/{}/reviews", substrate_pr_api_path,),
//))
//.respond_with(json_encoded(vec![github::Review {
//id: 1,
//user: Some(placeholder_user.clone()),
//state: Some(github::ReviewState::Approved),
//}])),
//);
//github_api.expect(
//Expectation::matching(request::method_path(
//"GET",
//format!(
//"/repos/{}/{}/commits/{}/status",
//org, substrate_repo_name, placeholder_sha
//),
//))
//.respond_with(json_encoded(github::CombinedStatus {
//state: github::StatusState::Success,
//statuses: vec![github::Status {
//id: 1,
//context: "DOES_NOT_MATTER".to_string(),
//state: github::StatusState::Success,
//}],
//})),
//);
//github_api.expect(
//Expectation::matching(request::method_path(
//"GET",
//format!(
//"/repos/{}/{}/commits/{}/check-runs",
//org, substrate_repo_name, placeholder_sha
//),
//))
//.respond_with(json_encoded(github::CheckRuns {
//check_runs: vec![github::CheckRun {
//id: 1,
//name: "DOES_NOT_MATTER".to_string(),
//status: github::CheckRunStatus::Completed,
//conclusion: Some(github::CheckRunConclusion::Success),
//head_sha: placeholder_sha.to_string(),
//}],
//})),
//);
//github_api.expect(
//Expectation::matching(request::method_path(
//"POST",
//format!("{}/comments", substrate_pr_issue_api_path,),
//))
//.respond_with(
//status_code(201)
//.append_header("Content-Type", "application/json")
//.body(serde_json::to_string(&json!({})).unwrap()),
//),
//);

//let placeholder_private_key = "-----BEGIN PRIVATE KEY-----
//MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDJETqse41HRBsc
//7cfcq3ak4oZWFCoZlcic525A3FfO4qW9BMtRO/iXiyCCHn8JhiL9y8j5JdVP2Q9Z
//IpfElcFd3/guS9w+5RqQGgCR+H56IVUyHZWtTJbKPcwWXQdNUX0rBFcsBzCRESJL
//eelOEdHIjG7LRkx5l/FUvlqsyHDVJEQsHwegZ8b8C0fz0EgT2MMEdn10t6Ur1rXz
//jMB/wvCg8vG8lvciXmedyo9xJ8oMOh0wUEgxziVDMMovmC+aJctcHUAYubwoGN8T
//yzcvnGqL7JSh36Pwy28iPzXZ2RLhAyJFU39vLaHdljwthUaupldlNyCfa6Ofy4qN
//ctlUPlN1AgMBAAECggEAdESTQjQ70O8QIp1ZSkCYXeZjuhj081CK7jhhp/4ChK7J
//GlFQZMwiBze7d6K84TwAtfQGZhQ7km25E1kOm+3hIDCoKdVSKch/oL54f/BK6sKl
//qlIzQEAenho4DuKCm3I4yAw9gEc0DV70DuMTR0LEpYyXcNJY3KNBOTjN5EYQAR9s
//2MeurpgK2MdJlIuZaIbzSGd+diiz2E6vkmcufJLtmYUT/k/ddWvEtz+1DnO6bRHh
//xuuDMeJA/lGB/EYloSLtdyCF6sII6C6slJJtgfb0bPy7l8VtL5iDyz46IKyzdyzW
//tKAn394dm7MYR1RlUBEfqFUyNK7C+pVMVoTwCC2V4QKBgQD64syfiQ2oeUlLYDm4
//CcKSP3RnES02bcTyEDFSuGyyS1jldI4A8GXHJ/lG5EYgiYa1RUivge4lJrlNfjyf
//dV230xgKms7+JiXqag1FI+3mqjAgg4mYiNjaao8N8O3/PD59wMPeWYImsWXNyeHS
//55rUKiHERtCcvdzKl4u35ZtTqQKBgQDNKnX2bVqOJ4WSqCgHRhOm386ugPHfy+8j
//m6cicmUR46ND6ggBB03bCnEG9OtGisxTo/TuYVRu3WP4KjoJs2LD5fwdwJqpgtHl
//yVsk45Y1Hfo+7M6lAuR8rzCi6kHHNb0HyBmZjysHWZsn79ZM+sQnLpgaYgQGRbKV
//DZWlbw7g7QKBgQCl1u+98UGXAP1jFutwbPsx40IVszP4y5ypCe0gqgon3UiY/G+1
//zTLp79GGe/SjI2VpQ7AlW7TI2A0bXXvDSDi3/5Dfya9ULnFXv9yfvH1QwWToySpW
//Kvd1gYSoiX84/WCtjZOr0e0HmLIb0vw0hqZA4szJSqoxQgvF22EfIWaIaQKBgQCf
//34+OmMYw8fEvSCPxDxVvOwW2i7pvV14hFEDYIeZKW2W1HWBhVMzBfFB5SE8yaCQy
//pRfOzj9aKOCm2FjjiErVNpkQoi6jGtLvScnhZAt/lr2TXTrl8OwVkPrIaN0bG/AS
//aUYxmBPCpXu3UjhfQiWqFq/mFyzlqlgvuCc9g95HPQKBgAscKP8mLxdKwOgX8yFW
//GcZ0izY/30012ajdHY+/QK5lsMoxTnn0skdS+spLxaS5ZEO4qvPVb8RAoCkWMMal
//2pOhmquJQVDPDLuZHdrIiKiDM20dy9sMfHygWcZjQ4WSxf/J7T9canLZIXFhHAZT
//3wc9h4G8BBCtWN2TN/LsGZdB
//-----END PRIVATE KEY-----"
//.as_bytes()
//.to_vec();

//let state = setup(
//Some(MainConfig {
//environment: "".to_string(),
//test_repo: "".to_string(),
//installation_login: bot_username.to_string(),
//webhook_secret: "".to_string(),
//webhook_port: "".to_string(),
//db_path: (&db_dir).path().display().to_string(),
//bamboo_token: "".to_string(),
//private_key: placeholder_private_key.clone(),
//matrix_homeserver: "".to_string(),
//matrix_access_token: "".to_string(),
//matrix_default_channel_id: "".to_string(),
//main_tick_secs: 0,
//bamboo_tick_secs: 0,
//matrix_silent: true,
//gitlab_hostname: "".to_string(),
//gitlab_project: "".to_string(),
//gitlab_job_name: "".to_string(),
//gitlab_private_token: "".to_string(),
//github_app_id: placeholder_id,
//}),
//Some(BotConfig {
//status_failure_ping: 0,
//issue_not_addressed_ping: 0,
//issue_not_assigned_to_pr_author_ping: 0,
//no_project_author_is_core_ping: 0,
//no_project_author_is_core_close_pr: 0,
//no_project_author_unknown_close_pr: 0,
//project_confirmation_timeout: 0,
//review_request_ping: 0,
//private_review_reminder_ping: 0,
//public_review_reminder_ping: 0,
//public_review_reminder_delay: 0,
//min_reviewers: 0,
//core_sorting_repo_name: "".to_string(),
//logs_room_id: "".to_string(),
//}),
//Some(matrix_bot::MatrixBot::new_placeholder_for_testing()),
//Some(gitlab_bot::GitlabBot::new_placeholder_for_testing()),
//Some(github_bot::GithubBot::new_for_testing(
//placeholder_private_key.clone(),
//bot_username,
//git_fetch_url,
//)),
//false,
//)
//.await
//.unwrap();

//handle_payload(
//github::Payload::IssueComment {
//action: github::IssueCommentAction::Created,
//comment: github::Comment {
//body: "bot merge".to_string(),
//user: Some(placeholder_user.clone()),
//},
//issue: github::Issue {
//number: substrate_pr_number,
//body: Some("".to_string()),
//html_url: substrate_pr_url,
//repository_url: Some(substrate_repository_url.to_string()),
//pull_request: Some(github::IssuePullRequest {}),
//repository: Some(substrate_repo.clone()),
//user: Some(placeholder_user.clone()),
//},
//},
//&state,
//)
//.await
//.unwrap();

//assert_snapshot!(read_snapshot((&log_dir).path().to_path_buf()));
//}

#[tokio::test]
async fn recover_outdated_master() {
	let log_dir = tempfile::tempdir().unwrap();
	flexi_logger::Logger::with_env_or_str("info")
		.log_to_file()
		.directory((&log_dir).path().to_path_buf())
		.duplicate_to_stdout(flexi_logger::Duplicate::All)
		.start()
		.unwrap();

	let bot_username = "bot";
	let placeholder_user = github::User {
		login: "foo".to_string(),
		type_field: Some(github::UserType::User),
	};
	let placeholder_sha = "MDEwOlJlcG9zaXRvcnkxMDk4NzI2MjA";
	let placeholder_id: usize = 1;

	let db_dir = tempfile::tempdir().unwrap();

	let git_daemon_dir = tempfile::tempdir().unwrap();
	//let git_daemon_dir = &*Box::leak(Box::new(tempfile::tempdir().unwrap()));
	println!("{:?}", git_daemon_dir);
	clean_directory(git_daemon_dir.path().to_path_buf());
	let git_daemon_port = get_available_port().unwrap();
	let git_fetch_url = &format!("git://127.0.0.1:{}", git_daemon_port);

	let org = "org";
	let teamleads_team_id = 1;
	let coredevs_team_id = 2;
	let substrate_repo_name = "substrate";
	let substrate_repo_dir =
		git_daemon_dir.path().join(org).join(substrate_repo_name);
	let substrate_user = &github::User {
		login: org.to_string(),
		type_field: Some(github::UserType::User),
	};
	let substrate_repo = github::Repository {
		name: substrate_repo_name.to_string(),
		full_name: Some(format!("{}/{}", org, substrate_repo_name)),
		owner: Some(substrate_user.clone()),
		html_url: "".to_string(),
	};
	fs::create_dir_all(&substrate_repo_dir).unwrap();
	run_cmd_with_dir(
		"git",
		&["init", "-b", "master"],
		&substrate_repo_dir,
		None,
	);
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
	fs::create_dir_all(&substrate_src_dir).unwrap();
	fs::write((&substrate_src_dir).join("main.rs"), "fn main() {}").unwrap();
	run_cmd_with_dir("git", &["add", "."], &substrate_repo_dir, None);
	run_cmd_with_dir(
		"git",
		&["commit", "-m", "init"],
		&substrate_repo_dir,
		None,
	);
	let substrate_head_sha = &get_cmd_output_with_dir(
		"git",
		&["rev-parse", "HEAD"],
		&substrate_repo_dir,
	);

	// Create PR branch
	run_cmd_with_dir(
		"git",
		&["checkout", "-b", "develop"],
		&substrate_repo_dir,
		Some(CmdConfiguration::SilentStderrStartingWith(&[
			"Switched to a new branch",
		])),
	);

	// Commit on master to make the PR branch outdated
	run_cmd_with_dir(
		"git",
		&["checkout", "master"],
		&substrate_repo_dir,
		Some(CmdConfiguration::SilentStderrStartingWith(&[
			"Switched to branch",
		])),
	);
	fs::write((&substrate_src_dir).join("foo.txt"), "").unwrap();
	run_cmd_with_dir(
		"git",
		&["commit", "-m", "update"],
		&substrate_repo_dir,
		None,
	);

	// Hold onto the git daemon process handle until the test is done
	let mut git_daemon_handle = tokio::process::Command::new("git")
		.arg("daemon")
		.arg(format!("--port={}", git_daemon_port))
		.arg("--base-path=.")
		.arg("--export-all")
		.kill_on_drop(true)
		.stdout(Stdio::null())
		.current_dir(git_daemon_dir.path())
		.spawn()
		.unwrap();

	let github_api = Server::run();
	let api_base_url = &github_api.url("").to_string();
	let api_base_url = &api_base_url[0..api_base_url.len() - 1];
	github::BASE_API_URL.set(api_base_url.to_owned()).unwrap();
	github_api.expect(
		Expectation::matching(request::method_path(
			"GET",
			"/app/installations",
		))
		.respond_with(json_encoded(vec![github::Installation {
			id: 1,
			account: github::User {
				login: bot_username.to_string(),
				type_field: Some(github::UserType::Bot),
			},
		}])),
	);
	github_api.expect(
		Expectation::matching(request::method_path(
			"POST",
			format!("/app/installations/{}/access_tokens", 1),
		))
		.respond_with(json_encoded(github::InstallationToken {
			token: "DOES_NOT_MATTER".to_string(),
			expires_at: None,
		})),
	);

	for (team_name, team_id) in &[
		(CORE_DEVS_GROUP, coredevs_team_id),
		(SUBSTRATE_TEAM_LEADS_GROUP, teamleads_team_id),
	] {
		github_api.expect(
			Expectation::matching(request::method_path(
				"GET",
				format!("/orgs/{}/teams/{}", org, team_name),
			))
			.times(0..)
			.respond_with(json_encoded(github::Team { id: *team_id })),
		);
		github_api.expect(
			Expectation::matching(request::method_path(
				"GET",
				format!("/teams/{}/members", team_id),
			))
			.times(0..)
			.respond_with(json_encoded(vec![placeholder_user.clone()])),
		);
	}

	let substrate_pr_number = 1;
	let substrate_repository_url =
		format!("https://github.com/{}/{}", org, substrate_repo_name);
	let substrate_pr_url =
		format!("{}/pull/{}", substrate_repository_url, substrate_pr_number);
	let substrate_pr_api_path = &format!(
		"/repos/{}/{}/pull/{}",
		org, substrate_repo_name, substrate_pr_number
	);
	let substrate_pr_issue_api_path = &format!(
		"/repos/{}/{}/issues/{}",
		org, substrate_repo_name, substrate_pr_number
	);
	let substrate_pr_api_url =
		&format!("{}/{}", api_base_url, substrate_pr_api_path);

	let substrate_dir_path_1: &'static PathBuf =
		&*Box::leak(Box::new(substrate_repo_dir.clone()));
	github_api.expect(
		Expectation::matching(request::method_path(
			"PUT",
			format!(
				"/repos/{}/{}/pulls/{}/merge",
				org, substrate_repo_name, substrate_pr_number
			),
		))
		.respond_with(move || {
			run_cmd_with_dir(
				"git",
				&["checkout", "develop"],
				substrate_dir_path_1,
				Some(CmdConfiguration::SilentStderrStartingWith(&[
					"Switched to branch",
				])),
			);
			let merge_output = get_cmd_output_with_dir(
				"git",
				&["merge", "master"],
				substrate_dir_path_1,
			);
			// Merge is only successful if contributor branch is up-to-date with master; simulates
			// the failure caused by Github API when the PR is outdated due to branch protection
			// rules.
			let result = if merge_output == "Already updated." {
				status_code(200)
					.append_header("Content-Type", "application/json")
					.body(serde_json::to_string(&json!({})).unwrap())
			} else {
				status_code(405)
					.append_header("Content-Type", "application/json")
					.body(
						serde_json::to_string(
							&json!({ "message": "Pull Request is not mergeable" }),
						)
						.unwrap(),
					)
			};
			run_cmd_with_dir(
				"git",
				&["merge", "--abort"],
				substrate_dir_path_1,
				Some(CmdConfiguration::SilentStderrStartingWith(&[
					"fatal: There is no merge to abort",
				])),
			);
			run_cmd_with_dir(
				"git",
				&["checkout", "master"],
				substrate_dir_path_1,
				Some(CmdConfiguration::SilentStderrStartingWith(&[
					"Switched to branch",
				])),
			);
			result
		}),
	);
	github_api.expect(
		Expectation::matching(request::method_path(
			"GET",
			format!("/orgs/{}/members/{}", org, (&placeholder_user).login),
		))
		.respond_with(
			status_code(204)
				.append_header("Content-Type", "application/json")
				.body(serde_json::to_string(&json!({})).unwrap()),
		),
	);
	github_api.expect(
		Expectation::matching(request::method_path(
			"GET",
			format!(
				"/repos/{}/{}/pulls/{}",
				org, substrate_repo_name, substrate_pr_number
			),
		))
		.respond_with(json_encoded(github::PullRequest {
			body: Some("".to_string()),
			number: substrate_pr_number,
			labels: vec![],
			mergeable: Some(true),
			html_url: substrate_pr_url.clone(),
			url: substrate_pr_api_url.clone(),
			user: Some(placeholder_user.clone()),
			repository: Some(substrate_repo.clone()),
			base: github::Base {
				ref_field: Some("master".to_string()),
				sha: Some(substrate_head_sha.to_string()),
				repo: Some(github::HeadRepo {
					name: substrate_repo_name.to_string(),
					owner: Some(substrate_user.clone()),
				}),
			},
			head: Some(github::Head {
				ref_field: Some("develop".to_string()),
				sha: Some(placeholder_sha.to_string()),
				repo: Some(github::HeadRepo {
					name: substrate_repo_name.to_string(),
					owner: Some(substrate_user.clone()),
				}),
			}),
		})),
	);
	github_api.expect(
		Expectation::matching(request::method_path(
			"GET",
			format!("/{}/reviews", substrate_pr_api_path,),
		))
		.respond_with(json_encoded(vec![github::Review {
			id: 1,
			user: Some(placeholder_user.clone()),
			state: Some(github::ReviewState::Approved),
		}])),
	);
	github_api.expect(
		Expectation::matching(request::method_path(
			"GET",
			format!(
				"/repos/{}/{}/commits/{}/status",
				org, substrate_repo_name, placeholder_sha
			),
		))
		.respond_with(json_encoded(github::CombinedStatus {
			state: github::StatusState::Success,
			statuses: vec![github::Status {
				id: 1,
				context: "DOES_NOT_MATTER".to_string(),
				state: github::StatusState::Success,
			}],
		})),
	);
	github_api.expect(
		Expectation::matching(request::method_path(
			"GET",
			format!(
				"/repos/{}/{}/commits/{}/check-runs",
				org, substrate_repo_name, placeholder_sha
			),
		))
		.respond_with(json_encoded(github::CheckRuns {
			check_runs: vec![github::CheckRun {
				id: 1,
				name: "DOES_NOT_MATTER".to_string(),
				status: github::CheckRunStatus::Completed,
				conclusion: Some(github::CheckRunConclusion::Success),
				head_sha: placeholder_sha.to_string(),
			}],
		})),
	);
	github_api.expect(
		Expectation::matching(request::method_path(
			"POST",
			format!("{}/comments", substrate_pr_issue_api_path,),
		))
		.respond_with(
			status_code(201)
				.append_header("Content-Type", "application/json")
				.body(serde_json::to_string(&json!({})).unwrap()),
		),
	);

	let placeholder_private_key = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDJETqse41HRBsc
7cfcq3ak4oZWFCoZlcic525A3FfO4qW9BMtRO/iXiyCCHn8JhiL9y8j5JdVP2Q9Z
IpfElcFd3/guS9w+5RqQGgCR+H56IVUyHZWtTJbKPcwWXQdNUX0rBFcsBzCRESJL
eelOEdHIjG7LRkx5l/FUvlqsyHDVJEQsHwegZ8b8C0fz0EgT2MMEdn10t6Ur1rXz
jMB/wvCg8vG8lvciXmedyo9xJ8oMOh0wUEgxziVDMMovmC+aJctcHUAYubwoGN8T
yzcvnGqL7JSh36Pwy28iPzXZ2RLhAyJFU39vLaHdljwthUaupldlNyCfa6Ofy4qN
ctlUPlN1AgMBAAECggEAdESTQjQ70O8QIp1ZSkCYXeZjuhj081CK7jhhp/4ChK7J
GlFQZMwiBze7d6K84TwAtfQGZhQ7km25E1kOm+3hIDCoKdVSKch/oL54f/BK6sKl
qlIzQEAenho4DuKCm3I4yAw9gEc0DV70DuMTR0LEpYyXcNJY3KNBOTjN5EYQAR9s
2MeurpgK2MdJlIuZaIbzSGd+diiz2E6vkmcufJLtmYUT/k/ddWvEtz+1DnO6bRHh
xuuDMeJA/lGB/EYloSLtdyCF6sII6C6slJJtgfb0bPy7l8VtL5iDyz46IKyzdyzW
tKAn394dm7MYR1RlUBEfqFUyNK7C+pVMVoTwCC2V4QKBgQD64syfiQ2oeUlLYDm4
CcKSP3RnES02bcTyEDFSuGyyS1jldI4A8GXHJ/lG5EYgiYa1RUivge4lJrlNfjyf
dV230xgKms7+JiXqag1FI+3mqjAgg4mYiNjaao8N8O3/PD59wMPeWYImsWXNyeHS
55rUKiHERtCcvdzKl4u35ZtTqQKBgQDNKnX2bVqOJ4WSqCgHRhOm386ugPHfy+8j
m6cicmUR46ND6ggBB03bCnEG9OtGisxTo/TuYVRu3WP4KjoJs2LD5fwdwJqpgtHl
yVsk45Y1Hfo+7M6lAuR8rzCi6kHHNb0HyBmZjysHWZsn79ZM+sQnLpgaYgQGRbKV
DZWlbw7g7QKBgQCl1u+98UGXAP1jFutwbPsx40IVszP4y5ypCe0gqgon3UiY/G+1
zTLp79GGe/SjI2VpQ7AlW7TI2A0bXXvDSDi3/5Dfya9ULnFXv9yfvH1QwWToySpW
Kvd1gYSoiX84/WCtjZOr0e0HmLIb0vw0hqZA4szJSqoxQgvF22EfIWaIaQKBgQCf
34+OmMYw8fEvSCPxDxVvOwW2i7pvV14hFEDYIeZKW2W1HWBhVMzBfFB5SE8yaCQy
pRfOzj9aKOCm2FjjiErVNpkQoi6jGtLvScnhZAt/lr2TXTrl8OwVkPrIaN0bG/AS
aUYxmBPCpXu3UjhfQiWqFq/mFyzlqlgvuCc9g95HPQKBgAscKP8mLxdKwOgX8yFW
GcZ0izY/30012ajdHY+/QK5lsMoxTnn0skdS+spLxaS5ZEO4qvPVb8RAoCkWMMal
2pOhmquJQVDPDLuZHdrIiKiDM20dy9sMfHygWcZjQ4WSxf/J7T9canLZIXFhHAZT
3wc9h4G8BBCtWN2TN/LsGZdB
-----END PRIVATE KEY-----"
		.as_bytes()
		.to_vec();

	let state = setup(
		Some(MainConfig {
			environment: "".to_string(),
			test_repo: "".to_string(),
			installation_login: bot_username.to_string(),
			webhook_secret: "".to_string(),
			webhook_port: "".to_string(),
			db_path: (&db_dir).path().display().to_string(),
			bamboo_token: "".to_string(),
			private_key: placeholder_private_key.clone(),
			matrix_homeserver: "".to_string(),
			matrix_access_token: "".to_string(),
			matrix_default_channel_id: "".to_string(),
			main_tick_secs: 0,
			bamboo_tick_secs: 0,
			matrix_silent: true,
			gitlab_hostname: "".to_string(),
			gitlab_project: "".to_string(),
			gitlab_job_name: "".to_string(),
			gitlab_private_token: "".to_string(),
			github_app_id: placeholder_id,
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
			core_sorting_repo_name: "".to_string(),
			logs_room_id: "".to_string(),
		}),
		Some(matrix_bot::MatrixBot::new_placeholder_for_testing()),
		Some(gitlab_bot::GitlabBot::new_placeholder_for_testing()),
		Some(github_bot::GithubBot::new_for_testing(
			placeholder_private_key.clone(),
			bot_username,
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
				user: Some(placeholder_user.clone()),
			},
			issue: github::Issue {
				number: substrate_pr_number,
				body: Some("".to_string()),
				html_url: substrate_pr_url,
				repository_url: Some(substrate_repository_url.to_string()),
				pull_request: Some(github::IssuePullRequest {}),
				repository: Some(substrate_repo.clone()),
				user: Some(placeholder_user.clone()),
			},
		},
		&state,
	)
	.await
	.unwrap();

	git_daemon_handle.kill().unwrap();
	assert_snapshot!(read_snapshot((&log_dir).path().to_path_buf()));
}
