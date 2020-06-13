// wengwengweng

mod browse;
mod bufs;

use browse::*;
use bufs::*;

use std::path::Path;
use std::collections::BTreeMap;
use std::process::Command;
use std::process::Stdio;

use dirty::*;
use math::*;
use gfx::*;
use input::*;

type ID = usize;

const SBAR_FONT_SIZE: f32 = 12.0;
const SBAR_COLOR: Color = rgba!(0, 0, 1, 1);
const SBAR_PADDING: Vec2 = vec2!(8, 6);
const SBAR_HEIGHT: f32 = SBAR_FONT_SIZE + SBAR_PADDING.y * 2.0;

const BUFBAR_FONT_SIZE: f32 = 10.0;
const BUFBAR_TAB_WIDTH: f32 = 144.0;
const BUFBAR_COLOR: Color = rgba!(1, 0, 0.5, 1);
const BUFBAR_PADDING: Vec2 = vec2!(8, 5);
const BUFBAR_HEIGHT: f32 = BUFBAR_FONT_SIZE + BUFBAR_PADDING.y * 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum View {
	Browser,
	Buffer,
}

trait Buffer: 'static {
	fn title(&self) -> String {
		if let Some(path) = self.path() {
			return path
				.file_name()
				.map(|s| s.to_string_lossy().to_string())
				.unwrap_or(String::new());
		}
		return String::new();
	}
	fn path(&self) -> Option<&Path> {
		return None;
	}
	fn event(&mut self, _: &mut Ctx, _: &input::Event) -> Result<()> {
		return Ok(());
	}
	fn update(&mut self, _: &mut Ctx) -> Result<()> {
		return Ok(());
	}
	fn draw(&self, _: &mut Gfx) -> Result<()> {
		return Ok(());
	}
	fn set_active(&mut self, _: bool) {}
	fn set_view_size(&mut self, _: f32, _: f32) {}

}

struct App {
	browser: FileBrowser,
	view: View,
	buffers: BTreeMap<ID, Box<dyn Buffer>>,
	last_buf_id: ID,
	cur_buf: Option<ID>,
	bufbar_offset: f32,
}

impl App {

	fn to_buf(&mut self, id: ID) {
		self.view = View::Buffer;
		self.cur_buf = Some(id);
	}

	fn new_buf(&mut self, mut b: impl Buffer) {

		let id = self.last_buf_id;

		b.set_active(true);

		self.buffers.insert(id, Box::new(b));
		self.last_buf_id += 1;
		self.to_buf(id);

	}

	fn open(&mut self, path: impl AsRef<Path>) -> Result<()> {

		let path = path.as_ref();

		for (id, buf) in &self.buffers {
			if Some(path) == buf.path() {
				self.to_buf(*id);
				return Ok(());
			}
		}

		if let Ok(ext) = fs::extname(path) {

			match ext.as_ref() {

				"png"
				| "jpg"
				=> return Ok(self.new_buf(ImgBuf::new(path))),

				"glb"
				| "obj"
				=> return Ok(self.new_buf(ModelBuf::new(path))),

				"mp3"
				| "ogg"
				| "wav"
				=> return Ok(self.new_buf(MusicBuf::new(path))),

				"blend"
				| "ase"
				| "pdf"
				| "app"
				| "mp4"
				=> return sysopen(path),

				_ => {},

			}

		}

		self.new_buf(TextBuf::new(path));

		return Ok(());

	}

}

impl State for App {

	fn init(_: &mut Ctx) -> Result<Self> {
		return Ok(Self {
			browser: FileBrowser::new(std::env::current_dir().map_err(|_| format!("failed to get current dir"))?)?,
			view: View::Browser,
			buffers: bmap![],
			last_buf_id: 0,
			cur_buf: None,
			bufbar_offset: 0.0,
		});
	}

	fn event(&mut self, d: &mut Ctx, e: &input::Event) -> Result<()> {

		let kmods = d.window.key_mods();

		match self.view {
			View::Buffer => {
				// ...
			},
			View::Browser => {
				match e {
					Event::KeyPress(k) => {
						match k {
							Key::Enter => {
								if let Some(file) = self.browser.enter() {
									self.open(file)?;
								}
							},
							_ => {},
						}
					},
					_ => {},
				}
				self.browser.event(d, e)?;
			},
		}

		match e {
			Event::KeyPress(k) => {
				match k {
					Key::Q if kmods.meta => d.window.quit(),
					Key::F if kmods.meta => d.window.toggle_fullscreen(),
					Key::Tab => {
						self.view = match self.view {
							View::Buffer => View::Browser,
							View::Browser => View::Buffer,
						};
					},
					_ => {},
				}
			},
			_ => {},
		}

		return Ok(());
	}

	fn update(&mut self, d: &mut Ctx) -> Result<()> {
		self.browser.update(d)?;
		return Ok(());
	}

	fn draw(&mut self, d: &mut Ctx) -> Result<()> {

		let gw = d.gfx.width() as f32;
		let gh = d.gfx.height() as f32;
		let top_left = d.gfx.coord(Origin::TopLeft);
		let top_right = d.gfx.coord(Origin::TopRight);
		let bot_right = d.gfx.coord(Origin::BottomRight);
		let mut y = 0.0;

		// status bar
		d.gfx.draw_within(
			top_left,
			top_right + vec2!(0, -SBAR_HEIGHT),
			|gfx| {

			gfx.draw(
				&shapes::rect(vec2!(0), vec2!(gfx.width(), -SBAR_HEIGHT))
					.fill(SBAR_COLOR)
			)?;

			gfx.draw_t(
				mat4!()
					.t2(vec2!(SBAR_PADDING.x, -SBAR_PADDING.y))
					,
				&shapes::text(&format!("{}", display_path(self.browser.path())))
					.size(SBAR_FONT_SIZE)
					.align(Origin::TopLeft)
					,
			)?;

			return Ok(());

		})?;

		y += SBAR_HEIGHT;

		// buffer bar
		if !self.buffers.is_empty() {

			d.gfx.draw_within(
				top_left + vec2!(0, -y),
				top_right + vec2!(0, -y - BUFBAR_HEIGHT),
				|gfx| {

				gfx.draw(
					&shapes::rect(vec2!(0), vec2!(gfx.width(), -BUFBAR_HEIGHT))
						.fill(BUFBAR_COLOR)
						,
				)?;

				gfx.push_t(mat4!().tx(self.bufbar_offset), |gfx| {

					for (i, (id, b)) in self.buffers.iter().enumerate() {

						let p1 = vec2!(i as f32 * BUFBAR_TAB_WIDTH, 0);
						let p2 = p1 + vec2!(BUFBAR_TAB_WIDTH, -BUFBAR_HEIGHT);

						if Some(*id) == self.cur_buf && self.view == View::Buffer {
							gfx.draw(
								&shapes::rect(p1, p2)
									.fill(BUFBAR_COLOR.darken(0.15))
									,
							)?;
						}

						gfx.draw_within(
							p1,
							p2,
							|gfx| {

							gfx.draw_t(
								mat4!()
									.t2(BUFBAR_PADDING * vec2!(1, -1))
									,
								&shapes::text(&b.title())
									.size(BUFBAR_FONT_SIZE)
									.align(Origin::TopLeft)
							)?;

							return Ok(());

						})?;

					}

					return Ok(());

				})?;

				return Ok(());

			})?;

			y += BUFBAR_HEIGHT;

		}

		d.gfx.draw_within(
			top_left + vec2!(0, -y),
			bot_right,
			|gfx| {

			match self.view {
				View::Buffer => {
					if let Some(id) = self.cur_buf {
						if let Some(buf) = self.buffers.get_mut(&id) {
							buf.set_view_size(gw, gh - y);
							buf.draw(gfx)?;
						}
					}
				},
				View::Browser => {
					self.browser.set_view_size(gw, gh - y);
					self.browser.draw(gfx)?;
				},
			}

			return Ok(());

		})?;

		return Ok(());

	}

}

fn sysopen(p: impl AsRef<Path>) -> Result<()> {
	Command::new("open")
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.arg(p.as_ref())
		.spawn()
		.map_err(|_| format!("failed to run command open"))?
		;
	return Ok(());
}

fn display_path(path: impl AsRef<Path>) -> String {

	let path = path.as_ref();
	let dpath = format!("{}", path.display());

	if let Some(home_dir) = dirs_next::home_dir() {
		return dpath.replace(&format!("{}", home_dir.display()), "~");
	}

	return dpath;

}

fn main() {
	if let Err(e) = launcher()
		.title("fopen")
		.resizable(true)
		.run::<App>() {
		elog!("{}", e);
	}
}

