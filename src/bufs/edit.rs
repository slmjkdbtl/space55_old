// wengwengweng

use std::path::Path;
use std::path::PathBuf;

use crate::*;

pub struct TextBuf {
	path: PathBuf,
}

impl TextBuf {
	pub fn new(path: impl AsRef<Path>) -> Self {
		return Self {
			path: path.as_ref().to_path_buf(),
		};
	}
}

impl Buffer for TextBuf {

	fn path(&self) -> Option<&Path> {
		return Some(&self.path);
	}

	fn draw(&self, gfx: &mut Gfx) -> Result<()> {
		return Ok(());
	}

}

