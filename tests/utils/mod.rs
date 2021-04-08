use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::{remove_dir_all, remove_file, File};
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

pub fn get_available_port() -> Option<u16> {
	for port in 1025..65535 {
		if let Ok(_) = TcpListener::bind(("127.0.0.1", port)) {
			return Some(port);
		}
	}

	None
}

pub fn read_snapshot(log_dir: PathBuf) -> String {
	let entry = log_dir.read_dir().unwrap().next().unwrap().unwrap();
	let mut file = File::open(entry.path()).unwrap();
	let mut buf = String::new();
	file.read_to_string(&mut buf).unwrap();
	buf
}

pub fn clean_directory(dir: PathBuf) {
	for f in dir.read_dir().unwrap() {
		let f = f.unwrap();
		let _ = if f.metadata().unwrap().is_dir() {
			remove_dir_all(f.path())
		} else {
			remove_file(f.path())
		};
	}
}

pub enum CmdConfiguration<'a> {
	SilentStderrStartingWith(&'a [&'a str]),
}

pub fn run_cmd_with_dir<'a, Cmd, Dir>(
	cmd: Cmd,
	args: &[&str],
	dir: Dir,
	conf: Option<CmdConfiguration<'a>>,
) where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path>,
{
	let mut init_cmd = Command::new(cmd);
	let cmd = init_cmd.args(args).current_dir(dir).stdout(Stdio::null());

	match conf {
		Some(CmdConfiguration::SilentStderrStartingWith(
			prefixes_to_ignore,
		)) => {
			let out = cmd
				.stderr(Stdio::piped())
				.spawn()
				.unwrap()
				.wait_with_output()
				.unwrap();
			let err = String::from_utf8_lossy(&out.stdout);
			let err = err.trim();
			if !err.is_empty() {
				for prefix_to_ignore in prefixes_to_ignore {
					if err.starts_with(prefix_to_ignore) {
						eprintln!("{}", err);
						break;
					}
				}
			};
		}
		_ => {
			cmd.spawn().unwrap().wait().unwrap();
		}
	}
}

pub fn get_cmd_output_with_dir<Cmd, Dir>(
	cmd: Cmd,
	args: &[&str],
	dir: Dir,
) -> String
where
	Cmd: AsRef<OsStr> + Display,
	Dir: AsRef<Path>,
{
	let out = Command::new(cmd)
		.args(args)
		.current_dir(dir)
		.stdout(Stdio::piped())
		.spawn()
		.unwrap()
		.wait_with_output()
		.unwrap();
	String::from_utf8_lossy(&out.stdout).trim().to_string()
}
