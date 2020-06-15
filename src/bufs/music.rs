// wengwengweng

use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;

use crate::*;

pub struct MusicPlayer {
	path: PathBuf,
	task: task::Task<Result<Vec<u8>>>,
	track: Option<audio::Track>,
	cover: Option<gfx::Texture>,
	view_size: Option<(f32, f32)>,
	title: Option<String>,
	artist: Option<String>,
	album: Option<String>,
}

impl MusicPlayer {

	pub fn new(path: impl AsRef<Path>) -> Result<Self> {

		let path = path.as_ref().to_path_buf();
		let path2 = path.clone();

		return Ok(Self {
			task: task::Task::exec(move || {
				return fs::read(path2);
			})?,
			path: path,
			track: None,
			cover: None,
			title: None,
			artist: None,
			album: None,
			view_size: None,
		});

	}

}

impl Buffer for MusicPlayer {

	fn path(&self) -> Option<&Path> {
		return Some(&self.path);
	}

	fn set_view_size(&mut self, w: f32, h: f32) {
		self.view_size = Some((w, h));
	}

	fn event(&mut self, d: &mut Ctx, e: &input::Event) -> Result<()> {

		let kmods = d.window.key_mods();

		match e {

			Event::KeyPress(k) => {
				match *k {
					Key::Space => {
						if let Some(track) = &self.track {
							if track.paused() {
								track.play();
							} else {
								track.pause();
							}
						}
					},
					_ => {},
				}
			}

			_ => {},

		}

		return Ok(());

	}

	fn update(&mut self, d: &mut Ctx) -> Result<()> {

		if let Some(data) = self.task.poll() {

			if let Ok(data) = data {

				let tag = id3::Tag::read_from(Cursor::new(&data[..])).ok();

				if let Some(tag) = &tag {

					self.album = tag.album().map(String::from);
					self.artist = tag.artist().map(String::from);
					self.title = tag.title().map(String::from);

					if let Some(p) = tag.pictures().next() {
						self.cover = gfx::Texture::from_bytes(d.gfx, &p.data).ok();
					}

				}

				if let Ok(track) = audio::Track::from_bytes(d.audio, &data) {
					track.play();
					self.track = Some(track);
				}

			}

		}

		return Ok(());

	}

	fn draw(&self, gfx: &mut Gfx) -> Result<()> {

		let (vw, vh) = self.view_size.unwrap_or((gfx.width() as f32, gfx.height() as f32));

		if let Some(cover) = &self.cover {

			let (w, h) = (cover.width() as f32, cover.height() as f32);
			let a1 = w / h;
			let a2 = vw / vh;

			let scale = if a1 > a2 {
				vw / w
			} else {
				vh / h
			};

			gfx.draw_t(
				mat4!()
					.t2(vec2!(vw, -vh) * 0.5)
					.s2(vec2!(scale))
					,
				&shapes::sprite(cover)
					,
			)?;
		}

		return Ok(());

	}

}

impl Drop for MusicPlayer {
	fn drop(&mut self) {
		if let Some(track) = self.track.take() {
			track.detach();
		}
		if let Some(cover) = self.cover.take() {
			cover.free();
		}
	}
}

