use crate::{error::*, Result};
use snafu::ResultExt;
use std::ffi::OsStr;
use std::fmt::Display;
use std::path::Path;
use std::process::{Output, Stdio};
use tokio::process::Command;

pub async fn run_cmd<Cmd, Dir>(
	cmd: Cmd,
	args: &[&str],
	dir: Dir,
) -> Result<Output>
where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path> + Display,
{
	before_cmd(&cmd, args, Some(&dir));

	#[allow(unused_mut)]
	let mut init_cmd = Command::new(cmd);
	let cmd = init_cmd.args(args).current_dir(dir).stderr(Stdio::piped());
	let result = cmd.output().await.context(Tokio)?;

	handle_cmd_result(cmd, result)
}

pub async fn run_cmd_in_cwd<Cmd>(cmd: Cmd, args: &[&str]) -> Result<Output>
where
	Cmd: AsRef<OsStr> + Display,
{
	before_cmd::<&Cmd, String>(&cmd, args, None);

	#[allow(unused_mut)]
	let mut init_cmd = Command::new(cmd);
	let cmd = init_cmd.args(args).stderr(Stdio::piped());
	let result = cmd.output().await.context(Tokio)?;

	handle_cmd_result(cmd, result)
}

pub async fn run_cmd_with_output<Cmd, Dir>(
	cmd: Cmd,
	args: &[&str],
	dir: Dir,
) -> Result<Output>
where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path> + Display,
{
	before_cmd(&cmd, args, Some(&dir));

	#[allow(unused_mut)]
	let mut init_cmd = Command::new(cmd);
	let cmd = init_cmd
		.args(args)
		.current_dir(dir)
		.stdin(Stdio::piped())
		.stderr(Stdio::piped());
	let result = cmd.output().await.context(Tokio)?;

	handle_cmd_result(cmd, result)
}

fn before_cmd<Cmd, Dir>(cmd: Cmd, args: &[&str], dir: Option<Dir>)
where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path> + Display,
{
	if let Some(dir) = dir {
		log::info!("Run {} {:?} in {}", cmd, args, dir)
	} else {
		log::info!("Run {} {:?} in the current directory", cmd, args)
	}
}

fn handle_cmd_result(cmd: &mut Command, result: Output) -> Result<Output> {
	if result.status.success() {
		Ok(result)
	} else {
		let err_output = String::from_utf8_lossy(&result.stderr);
		log::error!("{}", &err_output);
		Err(Error::CommandFailed {
			cmd: format!("{:?}", cmd),
			status_code: result.status.code(),
			err: err_output.to_string(),
		})
	}
}
