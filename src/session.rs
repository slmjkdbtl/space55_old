// wengwengweng

use std::path::PathBuf;
use serde::Serialize;
use serde::Deserialize;

use crate::*;

const PROJ: &str = "space55";
const ENTRY: &str = "session";

#[derive(Serialize, Deserialize)]
pub struct Session {
	pub path: PathBuf,
	pub bufs: Vec<PathBuf>,
}

impl Session {

	pub fn load() -> Result<Self> {
		return data::load(PROJ, ENTRY);
	}

	pub fn save(&self) -> Result<()> {
		return data::save(PROJ, ENTRY, self);
	}

}

