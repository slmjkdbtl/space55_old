// wengwengweng

use std::path::PathBuf;
use std::collections::BTreeMap;
use serde::Serialize;
use serde::Deserialize;

#[derive(Serialize, Deserialize)]
pub struct SaveData {
	pub path: PathBuf,
	pub bookmarks: BTreeMap<usize, PathBuf>,
}

