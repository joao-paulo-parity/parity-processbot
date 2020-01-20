pub type ProjectInfoMap = std::collections::HashMap<String, ProjectInfo>;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ProjectInfo {
	owner: Option<String>,
	delegated_reviewer: Option<String>,
	pub whitelist: Option<Vec<String>>,
	pub matrix_room_id: Option<String>,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct AuthorInfo {
	pub is_owner_or_delegate: bool,
	pub is_whitelisted: bool,
}

impl AuthorInfo {
	pub fn is_special(&self) -> bool {
		self.is_owner_or_delegate || self.is_whitelisted
	}
}

impl ProjectInfo {
	pub fn owner_or_delegate(&self) -> Option<&String> {
		self.delegated_reviewer.as_ref().or(self.owner.as_ref())
	}

	pub fn author_info(&self, login: &str) -> AuthorInfo {
		let is_owner = self.is_owner(login);
		let is_delegated_reviewer = self.is_delegated_reviewer(login);
		let is_whitelisted = self.is_whitelisted(login);

		AuthorInfo {
			is_owner_or_delegate: is_owner || is_delegated_reviewer,
			is_whitelisted,
		}
	}
	/// Checks if the owner of the project matches the login given.
	pub fn is_owner(&self, login: &str) -> bool {
		self.owner.as_deref().map_or(false, |owner| owner == login)
	}

	/// Checks if the delegated reviewer matches the login given.
	pub fn is_delegated_reviewer(&self, login: &str) -> bool {
		self.delegated_reviewer
			.as_deref()
			.map_or(false, |reviewer| reviewer == login)
	}

	/// Checks that the login is contained within the whitelist.
	pub fn is_whitelisted(&self, login: &str) -> bool {
		self.whitelist.as_ref().map_or(false, |whitelist| {
			whitelist.iter().any(|user| user == login)
		})
	}

	pub fn is_special(&self, login: &str) -> bool {
		self.is_owner(login)
			|| self.is_delegated_reviewer(login)
			|| self.is_whitelisted(login)
	}
}

pub fn projects_from_table(tab: toml::value::Table) -> ProjectInfoMap {
	tab.into_iter()
		.filter_map(|(key, val)| match val {
			toml::value::Value::Table(ref tab) => Some((
				key,
				ProjectInfo {
					owner: val
						.get("owner")
						.and_then(toml::value::Value::as_str)
						.map(str::to_owned),
					delegated_reviewer: tab
						.get("delegated_reviewer")
						.and_then(toml::value::Value::as_str)
						.map(str::to_owned),
					whitelist: tab
						.get("whitelist")
						.and_then(toml::value::Value::as_array)
						.map(|a| {
							a.iter()
								.filter_map(toml::value::Value::as_str)
								.map(str::to_owned)
								.collect::<Vec<String>>()
						}),
					matrix_room_id: tab
						.get("matrix_room_id")
						.and_then(toml::value::Value::as_str)
						.map(str::to_owned),
				},
			)),
			_ => None,
		})
		.collect()
}