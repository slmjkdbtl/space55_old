// wengwengweng

use std::path::Path;
use std::path::PathBuf;

use crate::*;

pub struct ModelViewer {
	shader: gfx::Shader<()>,
	path: PathBuf,
	task: task::Task<Result<ModelData>>,
	model: Option<Model>,
	rot: Vec2,
	pos: Vec2,
	scale: f32,
	resetting: bool,
	view_size: Option<(f32, f32)>,
}

impl ModelViewer {

	pub fn new(d: &mut Ctx, path: impl AsRef<Path>) -> Result<Self> {

		let path = path.as_ref().to_path_buf();
		let path2 = path.clone();

		return Ok(Self {
			path: path,
			task: task::Task::exec(move || {
				return Model::load_file(path2);
			})?,
			model: None,
			shader: gfx::Shader::from_frag(d.gfx, include_str!("normal.frag"))?,
			pos: vec2!(0),
			rot: vec2!(0),
			resetting: true,
			scale: 0.0,
			view_size: None,
		});

	}

}

impl Buffer for ModelViewer {

	fn path(&self) -> Option<&Path> {
		return Some(&self.path);
	}

	fn set_view_size(&mut self, w: f32, h: f32) {
		self.view_size = Some((w, h));
	}

	fn event(&mut self, d: &mut Ctx, e: &input::Event) -> Result<()> {

		use input::Event::*;

		match e {

			KeyPress(k) => {

				let mods = d.window.key_mods();

				match *k {
					Key::F => d.window.toggle_fullscreen(),
					Key::Esc => d.window.quit(),
					Key::Q if mods.meta => d.window.quit(),
					Key::Space => self.resetting = true,
					_ => {},
				}

			},

			Wheel(s, phase) => {

				if let input::ScrollPhase::Solid = phase {
					self.resetting = false;
				}

				if let Some(model) = &self.model {

					if !self.resetting {

						let bbox = model.bbox();
						let size = (bbox.max - bbox.min).len();
						let orig_scale = 480.0 / size;

						self.scale -= s.y * (1.0 / size);
						self.scale = self.scale.max(orig_scale * 0.1).min(orig_scale * 3.2);

					}

				}

			},

			MouseMove(delta) => {

				if d.window.mouse_down(Mouse::Left) {

					self.resetting = false;
					self.rot += *delta;

					if self.rot.x >= 360.0 {
						self.rot.x = self.rot.x - 360.0;
					}

					if self.rot.x <= -360.0 {
						self.rot.x = self.rot.x + 360.0;
					}

					if self.rot.y >= 360.0 {
						self.rot.y = self.rot.y - 360.0;
					}

					if self.rot.y <= -360.0 {
						self.rot.y = self.rot.y + 360.0;
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
				self.model = Some(Model::from_data(d.gfx, data)?);
			}
		}

		if let Some(model) = &self.model {

			let dt = d.app.dt().as_secs_f32();
			let move_speed = 480.0;

			if d.window.key_down(Key::A) {
				self.resetting = false;
				self.pos.x += move_speed * dt;
			}

			if d.window.key_down(Key::D) {
				self.resetting = false;
				self.pos.x -= move_speed * dt;
			}

			if d.window.key_down(Key::W) {
				self.resetting = false;
				self.pos.y -= move_speed * dt;
			}

			if d.window.key_down(Key::S) {
				self.resetting = false;
				self.pos.y += move_speed * dt;
			}

			if self.resetting {

				let bbox = model.bbox();
				let size = (bbox.max - bbox.min).len();

				let dest_rot = vec2!(0);
				let dest_pos = vec2!(0);
				let dest_scale = 480.0 / size;
				let t = dt * 4.0;

				self.rot = self.rot.lerp(dest_rot, t);
				self.pos = self.pos.lerp(dest_pos, t);
				self.scale = self.scale.lerp(dest_scale, t);

			}

		}

		return Ok(());

	}

	fn draw(&self, gfx: &mut Gfx) -> Result<()> {

		let (vw, vh) = self.view_size.unwrap_or((gfx.width() as f32, gfx.height() as f32));

		if let Some(model) = &self.model {

			let center = model.center();

			gfx.push_t(mat4!()
				.t2(self.pos + vec2!(vw, vh) * vec2!(0.5, -0.5))
				.s3(vec3!(self.scale))
				.ry(self.rot.x.to_radians())
				.rx(self.rot.y.to_radians())
				.t3(-center)
			, |gfx| {

				gfx.draw_with(&self.shader, &(), |gfx| {
					gfx.draw(
						&shapes::model(&model)
					)?;
					return Ok(());
				})?;

				return Ok(());

			})?;

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

