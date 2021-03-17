use crate::{error::*, Result};
use snafu::ResultExt;
use std::ffi::OsStr;
use std::fmt::Display;
use std::path::Path;
use std::process::{Output, Stdio};
use tokio::process::Command;

#[derive(PartialEq)]
pub struct CommandMessages {
	pub cmd_display: Option<String>,
	pub on_failure: Option<String>,
}

#[derive(PartialEq)]
pub enum CommandMessage {
	Enabled,
	SubstituteFor(CommandMessages),
}

pub async fn run_cmd<Cmd, Dir>(
	cmd: Cmd,
	args: &[&str],
	dir: Dir,
	logging: CommandMessage,
) -> Result<Output>
where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path> + Display,
{
	cmd_display(&cmd, args, Some(&dir), &logging);

	#[allow(unused_mut)]
	let mut init_cmd = Command::new(cmd);
	let cmd = init_cmd.args(args).current_dir(dir).stderr(Stdio::piped());
	let result = cmd.output().await.context(Tokio)?;

	handle_cmd_result(cmd, result, &logging)
}

pub async fn run_cmd_in_cwd<Cmd>(
	cmd: Cmd,
	args: &[&str],
	logging: CommandMessage,
) -> Result<Output>
where
	Cmd: AsRef<OsStr> + Display,
{
	cmd_display::<&Cmd, String>(&cmd, args, None, &logging);

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
	logging: CommandMessage,
) -> Result<Output>
where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path> + Display,
{
	cmd_display(&cmd, args, Some(&dir), &logging);

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

fn cmd_display<Cmd, Dir>(
	cmd: Cmd,
	args: &[&str],
	dir: Option<Dir>,
	logging: &CommandMessage,
) where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path> + Display,
{
	match logging {
		CommandMessage::Enabled => {
			if let Some(dir) = dir {
				log::info!("Run {} {:?} in {}", cmd, args, dir);
			} else {
				log::info!("Run {} {:?} in the current directory", cmd, args);
			}
		}
		CommandMessage::SubstituteFor(CommandMessages {
			cmd_display, ..
		}) => {
			if let Some(cmd_display) = cmd_display {
				log::info!("{}", cmd_display);
			}
		}
	};
}

fn handle_cmd_result(
	cmd: &mut Command,
	result: Output,
	logging: &CommandMessage,
) -> Result<Output> {
	if result.status.success() {
		Ok(result)
	} else {
		let err_msg = match logging {
			CommandMessage::Enabled => {
				let err_output = String::from_utf8_lossy(&result.stderr);
				if err_output.is_empty() {
					None
				} else {
					log::error!("{}", err_output);
					Some(err_output.to_string())
				}
			}
			CommandMessage::SubstituteFor(CommandMessages {
				on_failure,
				..
			}) => {
				if let Some(on_failure) = on_failure {
					log::error!("{}", on_failure);
					Some(on_failure.to_string())
				} else {
					None
				}
			}
		};

		let cmd_display = match logging {
			CommandMessage::SubstituteFor(CommandMessages {
				cmd_display,
				..
			}) => cmd_display.as_ref().map(|display| display.to_string()),
			_ => None,
		}
		.unwrap_or_else(|| format!("{:?}", cmd));

		Err(Error::CommandFailed {
			cmd: cmd_display,
			status_code: result.status.code(),
			err: err_msg.unwrap_or_else(|| "no output".to_string()),
		})
	}
}
