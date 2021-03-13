use std::net::TcpListener;

pub fn get_available_port() -> Option<u16> {
	for port in 1025..65535 {
		if let Ok(_) = TcpListener::bind(("127.0.0.1", port)) {
			return Some(port);
		}
	}

	None
}
