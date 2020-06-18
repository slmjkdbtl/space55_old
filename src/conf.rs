// wengwengweng

use std::collections::BTreeMap;
use std::path::PathBuf;
use serde::Serialize;
use serde::Deserialize;

use crate::*;

const FNAME: &str = ".space55.conf";

#[derive(Serialize, Deserialize)]
pub struct Conf {
	pub width: i32,
	pub height: i32,
	pub bookmarks: BTreeMap<usize, PathBuf>,
}

impl Conf {

	pub fn load() -> Result<Self> {

		let home = dirs_next::home_dir()
			.ok_or_else(|| format!("failed to get home dir"))?;
		let path = home.join(FNAME);
		let content = std::fs::read_to_string(&path)
			.map_err(|_| format!("failed to read {}", path.display()))?;

		return toml::from_str::<Self>(&content)
			.map_err(|_| format!("failed to parse conf"));

	}

}

impl Default for Conf {
	fn default() -> Self {
		return Self {
			width: 960,
			height: 640,
			bookmarks: bmap![],
		};
	}
}

