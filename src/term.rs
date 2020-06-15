// wengwengweng

use std::sync::mpsc;
use std::thread;
use crate::*;
use kit::textedit::Input;

pub struct Term {
	view_size: Option<(f32, f32)>,
	input: Input,
	output: String,
	cmd_rx: Option<mpsc::Receiver<u8>>,
}

impl Term {

	pub fn new() -> Self {
		return Self {
			view_size: None,
			input: Input::new(),
			output: String::new(),
			cmd_rx: None,
		};
	}

	pub fn exec(&mut self, cmd: &str) -> Result<()> {

		let (tx, rx) = mpsc::channel();
		let cmd = cmd.to_string();

		self.output = String::new();
		self.cmd_rx = Some(rx);

		thread::spawn(move || {

			let res: Result<()> = || -> Result<()> {

				let mut child = Command::new("fish")
					.arg("-c")
					.arg(&cmd)
					.stdin(Stdio::piped())
					.stdout(Stdio::piped())
					.stderr(Stdio::piped())
					.spawn()
					.map_err(|_| format!("failed to execute '{}'", cmd))?;

				use std::io::Read;
				use std::io::Write;

				let stdin = child.stdin.take().ok_or_else(|| format!("stdin"))?;
				let stdout = child.stdout.take().ok_or_else(|| format!("stdout"))?;
				let stderr = child.stderr.take().ok_or_else(|| format!("stderr"))?;
				let mut stdout_b = stdout.bytes();
				let mut stderr_b = stderr.bytes();

				while let Some(b) = stdout_b.next() {
					if let Ok(b) = b {
						tx.send(b).map_err(|_| format!("failed to send byte"))?;
					}
				}

				while let Some(b) = stderr_b.next() {
					if let Ok(b) = b {
						tx.send(b).map_err(|_| format!("failed to send byte"))?;
					}
				}

				return Ok(());

			}();

			if let Err(e) = res {
				elog!("{}", e);
			}

		});

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

		if let Some(cmd_rx) = &self.cmd_rx {
			for b in cmd_rx.try_iter() {
				self.output.push(b as char);
			}
		}

		return Ok(());

	}

	pub fn draw(&self, gfx: &mut Gfx) -> Result<()> {

		let cmd = self.input.content().to_string();

		gfx.draw(
			&shapes::text(&format!("> {}", cmd))
				.align(Origin::TopLeft)
				.size(16.0)
		)?;

		gfx.draw_t(
			mat4!()
				.ty(-16.0)
				,
			&shapes::text(&self.output)
				.align(Origin::TopLeft)
				.size(16.0)
				,
		)?;

		return Ok(());

	}
}

