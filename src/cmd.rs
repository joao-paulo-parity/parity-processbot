use crate::{error::*, Result};
use snafu::ResultExt;
use std::ffi::OsStr;
use std::fmt::Display;
use std::path::Path;
use std::process::{Output, Stdio};
use tokio::process::Command;

#[derive(PartialEq)]
pub struct CommandLoggingMessages {
	pub before_cmd: String,
	pub on_failure: String,
}

#[derive(PartialEq)]
pub enum CommandLogging {
	Enabled,
	SubstituteFor(CommandLoggingMessages),
}

pub async fn run_cmd<Cmd, Dir>(
	cmd: Cmd,
	args: &[&str],
	dir: Dir,
	logging: CommandLogging,
) -> Result<Output>
where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path> + Display,
{
	before_cmd(&cmd, args, Some(&dir), &logging);

	#[allow(unused_mut)]
	let mut init_cmd = Command::new(cmd);
	let cmd = init_cmd.args(args).current_dir(dir).stderr(Stdio::piped());
	let result = cmd.output().await.context(Tokio)?;

	handle_cmd_result(cmd, result, &logging)
}

pub async fn run_cmd_in_cwd<Cmd>(
	cmd: Cmd,
	args: &[&str],
	logging: CommandLogging,
) -> Result<Output>
where
	Cmd: AsRef<OsStr> + Display,
{
	before_cmd::<&Cmd, String>(&cmd, args, None, &logging);

	#[allow(unused_mut)]
	let mut init_cmd = Command::new(cmd);
	let cmd = init_cmd.args(args).stderr(Stdio::piped());
	let result = cmd.output().await.context(Tokio)?;

	handle_cmd_result(cmd, result, &logging)
}

pub async fn run_cmd_with_output<Cmd, Dir>(
	cmd: Cmd,
	args: &[&str],
	dir: Dir,
	logging: CommandLogging,
) -> Result<Output>
where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path> + Display,
{
	before_cmd(&cmd, args, Some(&dir), &logging);

	#[allow(unused_mut)]
	let mut init_cmd = Command::new(cmd);
	let cmd = init_cmd
		.args(args)
		.current_dir(dir)
		.stdin(Stdio::piped())
		.stderr(Stdio::piped());
	let result = cmd.output().await.context(Tokio)?;

	handle_cmd_result(cmd, result, &logging)
}

fn before_cmd<Cmd, Dir>(
	cmd: Cmd,
	args: &[&str],
	dir: Option<Dir>,
	logging: &CommandLogging,
) where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path> + Display,
{
	match logging {
		CommandLogging::Enabled => {
			if let Some(dir) = dir {
				log::info!("Run {} {:?} in {}", cmd, args, dir);
			} else {
				log::info!("Run {} {:?} in the current directory", cmd, args);
			}
		}
		CommandLogging::SubstituteFor(CommandLoggingMessages {
			before_cmd,
			..
		}) => {
			log::info!("{}", before_cmd);
		}
	};
}

fn handle_cmd_result(
	cmd: &mut Command,
	result: Output,
	logging: &CommandLogging,
) -> Result<Output> {
	if result.status.success() {
		Ok(result)
	} else {
		let err_msg = match logging {
			CommandLogging::Enabled => {
				let err_output = String::from_utf8_lossy(&result.stderr);
				log::info!("{}", err_output);
				err_output.to_string()
			}
			CommandLogging::SubstituteFor(CommandLoggingMessages {
				on_failure,
				..
			}) => {
				log::info!("{}", on_failure);
				on_failure.to_string()
			}
		};

		Err(Error::CommandFailed {
			cmd: format!("{:?}", cmd),
			status_code: result.status.code(),
			err: err_msg,
		})
	}
}
