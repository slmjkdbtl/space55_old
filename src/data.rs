// wengwengweng

use std::path::PathBuf;
use serde::Serialize;
use serde::Deserialize;

#[derive(Serialize, Deserialize)]
struct SaveData {
	path: PathBuf,
	bookmarks: Vec<PathBuf>,
}

