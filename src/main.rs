use futures_util::future::FutureExt;
use rocksdb::DB;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::process::Command;

use parity_processbot::{
	config::{BotConfig, MainConfig},
	github_bot, matrix_bot,
	server::*,
	webhook::*,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let clone = Command::new("git")
		.arg("clone")
		.arg("https://github.com/paritytech/polkadot.git")
		.arg("repo")
		.spawn()
		.expect("spawn clone")
		.then(|_| {
			Command::new("cd")
				.arg("repo")
				.arg("&&")
				.arg("cargo")
				.arg("update")
				.arg("-p")
				.arg("sp-io")
				.spawn()
				.expect("spawn update")
				.then(|_| {
					Command::new("git")
						.arg("commit")
						.arg("-a")
						.arg("-m")
						.arg("'Update substrate'")
						.spawn()
						.expect("spawn commit")
						.then(|_| {
							Command::new("git")
								.arg("push")
								.spawn()
								.expect("spawn push")
								.then(|_| {
									Command::new("rm")
										.arg("-rf")
										.arg("repo")
										.spawn()
										.expect("spawn repo")
								})
						})
				})
		});

	// Make sure our child succeeded in spawning and process the result
	let future = clone; //.then(|_| cd).then(|_| update).then(|_| commit).then(|_| rm);

	// Await until the future (and the command) completes
	let status = future.await?;
	println!("the command exited with: {}", status);

	Ok(())

	/*
	match run().await {
		Err(error) => panic!("{}", error),
		_ => Ok(()),
	}
	*/
}

async fn run() -> anyhow::Result<()> {
	let config = MainConfig::from_env();
	env_logger::from_env(env_logger::Env::default().default_filter_or("info"))
		.init();

	let db = DB::open_default(&config.db_path)?;

	log::info!(
		"Connecting to Matrix homeserver {}",
		config.matrix_homeserver,
	);
	let matrix_bot = matrix_bot::MatrixBot::new_with_token(
		&config.matrix_homeserver,
		&config.matrix_access_token,
		&config.matrix_default_channel_id,
		config.matrix_silent,
	)?;

	log::info!("Connecting to Github account {}", config.installation_login);
	let github_bot = github_bot::GithubBot::new(
		config.private_key.clone(),
		&config.installation_login,
	)
	.await?;

	// the bamboo queries can take a long time so only wait for it
	// on launch. subsequently update in the background.
	/*
	{
		let db_write = db.write();
		if db_write.get(BAMBOO_DATA_KEY).ok().flatten().is_none() {
			log::info!("Waiting for Bamboo data (may take a few minutes)");
			match bamboo::github_to_matrix(&config.bamboo_token) {
				Ok(h) => db_write
					.put(
						BAMBOO_DATA_KEY,
						bincode::serialize(&h).expect("serialize bamboo"),
					)
					.expect("put bamboo"),
				Err(e) => log::error!("Bamboo error: {}", e),
			}
		}
	}
	*/

	// let config_clone = config.clone();
	//	let db_clone = db.clone();
	//
	/*
	std::thread::spawn(move || loop {
		{
			let db_write = db_clone.write();
			match bamboo::github_to_matrix(&config_clone.bamboo_token) {
				Ok(h) => {
					db_write
						.put(
							BAMBOO_DATA_KEY,
							bincode::serialize(&h).expect("serialize bamboo"),
						)
						.expect("put bamboo");
				},
				Err(e) => log::error!("Bamboo error: {}", e),
			}
		}
		std::thread::sleep(Duration::from_secs(config_clone.bamboo_tick_secs));
	});
	*/

	let app_state = Arc::new(AppState {
		db: db,
		github_bot: github_bot,
		matrix_bot: matrix_bot,
		bot_config: BotConfig::from_env(),
		webhook_secret: config.webhook_secret,
		environment: config.environment,
		test_repo: config.test_repo,
	});

	let socket = SocketAddr::new(
		IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
		config.webhook_port.parse::<u16>().expect("webhook port"),
	);

	init_server(socket, app_state).await
}

#[cfg(test)]
mod tests {
	use regex::Regex;

	#[test]
	fn test_replace_whitespace_in_toml_key() {
		let mut s = String::from("[Smart Contracts Ok]\nwhitelist = []");
		let re = Regex::new(
			r"^\[((?:[[:word:]]|[[:punct:]])*)[[:blank:]]((?:[[:word:]]|[[:punct:]])*)",
		)
		.unwrap();
		while re.captures_iter(&s).count() > 0 {
			s = dbg!(re.replace_all(&s, "[$1-$2").to_string());
		}
		assert_eq!(&s, "[Smart-Contracts-Ok]\nwhitelist = []");
	}
}
