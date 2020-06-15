// wengwengweng

use std::path::PathBuf;
use std::collections::HashMap;
use serde::Serialize;
use serde::Deserialize;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conf {
	width: i32,
	height: i32,
	file_types: Vec<FileType>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileType {
	name: String,
	detect: String,
	comment: String,
}

