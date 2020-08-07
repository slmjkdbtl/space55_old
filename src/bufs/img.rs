// wengwengweng

use std::path::Path;
use std::path::PathBuf;

use crate::*;

pub struct ImageViewer {
	path: PathBuf,
	task: task::Task<Result<Vec<u8>>>,
	tex: Option<gfx::Texture>,
	view_size: Option<(f32, f32)>,
	pos: Vec2,
	scale: f32,
}

impl ImageViewer {

	pub fn new(path: impl AsRef<Path>) -> Result<Self> {

		let path = path.as_ref().to_path_buf();
		let path2 = path.clone();

		return Ok(Self {
			task: task::Task::new(move || {
				return fs::read(path2);
			})?,
			path: path,
			tex: None,
			view_size: None,
			pos: vec2!(),
			scale: 1.0,
		});

	}

}

impl Buffer for ImageViewer {

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
						self.pos = vec2!(0);
						self.scale = 1.0;
					},
					_ => {},
				}
			}

			Event::Wheel(d, p) => {

				if let input::ScrollPhase::Solid = p {

					if kmods.alt {
						self.scale -= d.y * 0.01;
						self.scale = self.scale.max(0.3).min(3.0);
					} else {
						self.pos += vec2!(d.x, d.y);
						self.pos = self.pos.clamp(vec2!(-200), vec2!(200));
					}

				}

			},

			_ => {},

		}

		return Ok(());

	}

	fn update(&mut self, d: &mut Ctx) -> Result<()> {

		if let Some(data) = self.task.poll() {
			if let Ok(data) = data {
				self.tex = gfx::Texture::from_bytes(d.gfx, &data).ok();
			}
		}

		return Ok(());

	}

	fn draw(&self, gfx: &mut Gfx) -> Result<()> {

		let (vw, vh) = self.view_size.unwrap_or((gfx.width() as f32, gfx.height() as f32));

		if let Some(tex) = &self.tex {
			gfx.draw_t(
				mat4!()
					.t2(vec2!(vw, -vh) * 0.5 + self.pos)
					.s2(vec2!(self.scale))
					,
				&shapes::sprite(tex)
					,
			)?;
		} else {
			gfx.draw_t(
				mat4!()
					.t2(vec2!(24, -24))
					,
				&shapes::text("loading...")
					.size(16.0)
			)?;
		}

		return Ok(());

	}

}

