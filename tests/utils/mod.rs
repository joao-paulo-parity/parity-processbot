use std::fs::{remove_dir_all, remove_file, File};
use std::io::Read;
use std::net::TcpListener;
use std::path::PathBuf;

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
