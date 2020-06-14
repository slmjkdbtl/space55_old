// wengwengweng

use crate::*;
use kit::textedit::Input;

enum Output {
	Success(String),
	Error(String),
}

pub struct Term {
	view_size: Option<(f32, f32)>,
	input: Input,
	output: Option<Output>,
}

impl Term {

	pub fn new() -> Self {
		return Self {
			view_size: None,
			input: Input::new(),
			output: None,
		};
	}

	pub fn exec(&mut self, cmd: &str) -> Result<()> {

		let mut scmd = cmd.split(' ');

		if let Some(program) = scmd.next() {

			let out = Command::new(program)
				.args(&scmd.collect::<Vec<&str>>())
				.output()
				.map_err(|_| format!(r#"failed to exec cmd "{}""#, cmd))?;

			if out.status.success() {
				self.output = Some(Output::Success(String::from_utf8_lossy(&out.stdout).to_string()));
			} else {
				self.output = Some(Output::Error(String::from_utf8_lossy(&out.stderr).to_string()));
			}

		}

		return Ok(());

	}

	pub fn set_view_size(&mut self, w: f32, h: f32) {
		self.view_size = Some((w, h));
	}

	pub fn event(&mut self, d: &mut Ctx, e: &Event) -> Result<()> {

		match e {
			Event::KeyPress(k) => {
				match *k {
					Key::Enter => {
						let cmd = self.input.content().to_string();
						self.input = Input::new();
						self.exec(&cmd)?;
					},
					_ => {},
				}
			},
			Event::KeyPressRepeat(k) => {
				match *k {
					Key::Backspace => self.input.del(),
					_ => {},
				}
			},
			Event::CharInput(ch) => {
				self.input.insert(*ch);
			},
			_ => {},
		}

		return Ok(());

	}

	pub fn update(&mut self, d: &mut Ctx) -> Result<()> {
		return Ok(());
	}

	pub fn draw(&self, gfx: &mut Gfx) -> Result<()> {

		let cmd = self.input.content().to_string();

		gfx.draw(
			&shapes::text(&format!("> {}", cmd))
				.align(Origin::TopLeft)
				.size(16.0)
		)?;

		if let Some(out) = &self.output {

			let (c, s) = match out {
				Output::Success(s) => (rgba!(1), s),
				Output::Error(s) => (rgba!(1, 0, 0, 1), s),
			};

			gfx.draw_t(
				mat4!()
					.ty(-16.0)
					,
				&shapes::text(s)
					.align(Origin::TopLeft)
					.size(16.0)
					.color(c)
			)?;
		}

		return Ok(());

	}
}

