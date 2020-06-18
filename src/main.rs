// wengwengweng

mod browse;
mod bufs;
mod term;
mod session;
mod conf;

use browse::*;
use bufs::*;
use term::*;
use session::*;
use conf::*;

use std::mem;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use std::time::Duration;
use std::collections::VecDeque;
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
const BUFBAR_TAB_WIDTH: f32 = 160.0;
const BUFBAR_COLOR: Color = rgba!(1, 0, 0.5, 1);
const BUFBAR_PADDING: Vec2 = vec2!(8, 5);
const BUFBAR_HEIGHT: f32 = BUFBAR_FONT_SIZE + BUFBAR_PADDING.y * 2.0;

const LOG_SIZE: usize = 5;
const LOG_LIFE: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum View {
	Browser,
	Buffer,
	Term,
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
	fn modified(&self) -> bool {
		return false;
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
	fn busy(&self) -> bool {
		return false;
	}
	fn closable(&self) -> bool {
		return true;
	}
	fn close(&mut self) {}
	fn set_active(&mut self, _: bool) {}
	fn set_view_size(&mut self, _: f32, _: f32) {}
	fn log(&mut self) -> Option<&mut Vec<Msg>> {
		return None;
	}

}

#[derive(Clone, Debug, PartialEq)]
struct Msg {
	r#type: MsgType,
	msg: String,
	time: Instant,
}

impl Msg {

	fn new(t: MsgType, m: &str) -> Self {
		return Self {
			r#type: t,
			msg: m.to_string(),
			time: Instant::now(),
		};
	}

	fn info(m: &str) -> Self {
		return Self::new(MsgType::Info, m);
	}

	fn success(m: &str) -> Self {
		return Self::new(MsgType::Success, m);
	}

	fn error(m: &str) -> Self {
		return Self::new(MsgType::Error, m);
	}

	fn age(&self) -> Duration {
		return self.time.elapsed();
	}

}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MsgType {
	Info,
	Error,
	Success,
}

struct App {
	browser: FileBrowser,
	term: Term,
	view: View,
	buffers: BTreeMap<ID, Box<dyn Buffer>>,
	last_buf_id: ID,
	cur_buf: Option<ID>,
	bufbar_offset: f32,
	bookmarks: BTreeMap<ID, PathBuf>,
	log: VecDeque<Msg>,
}

impl App {

	fn cur_path(&self) -> &Path {
		return match self.view {
			View::Buffer => {
				return self.cur_buf()
					.map(|buf| buf.path())
					.flatten()
					.unwrap_or(self.browser.path());
			},
			View::Browser => {
				return self.browser.path();
			},
			View::Term => {
				return self.browser.path();
			},
		};
	}

	fn cur_buf(&self) -> Option<&Box<dyn Buffer>> {
		if let Some(id) = self.cur_buf {
			return self.buffers.get(&id);
		}
		return None;
	}

	fn cur_buf_mut(&mut self) -> Option<&mut Box<dyn Buffer>> {
		if let Some(id) = self.cur_buf {
			return self.buffers.get_mut(&id);
		}
		return None;
	}

	fn to_buf(&mut self, id: ID) {
		self.view = View::Buffer;
		self.cur_buf = Some(id);
	}

	fn to_buf_n(&mut self, n: usize) {

		let ids = self.buffers.keys();

		if let Some(id) = ids.skip(n).next() {
			self.to_buf(*id);
		}

	}

	fn get_buf_n(&self, id: ID) -> Option<usize> {
		return self.buffers
			.keys()
			.position(|id2| *id2 == id);
	}

	fn to_prev_buf(&mut self) {

		if let Some(id) = self.cur_buf {
			let n = self.get_buf_n(id);
			if let Some(n) = n {
				if n > 0 {
					self.to_buf_n(n - 1);
				} else {
					self.to_buf_n(self.buffers.len() - 1);
				}
			}
		} else {
			self.cur_buf = self.buffers.keys().rev().next().cloned();
		}

	}

	fn to_next_buf(&mut self) {

		if let Some(id) = self.cur_buf {
			let n = self.get_buf_n(id);
			if let Some(n) = n {
				if n < self.buffers.len() - 1 {
					self.to_buf_n(n + 1);
				} else {
					self.to_buf_n(0);
				}
			}
		} else {
			self.cur_buf = self.buffers.keys().next().cloned();
		}

	}

	fn close_buf(&mut self, id: ID) {

		if let Some(buf) = self.buffers.get_mut(&id) {
			if !buf.closable() {
				return;
			}
			buf.close();
		}

		if Some(id) == self.cur_buf {
			if let Some(n) = self.get_buf_n(id) {
				if n > 0 {
					self.to_buf_n(n - 1);
				} else {
					self.to_buf_n(n + 1);
				}
			}
		}

		self.buffers.remove(&id);

		if self.view == View::Buffer {
			if self.buffers.is_empty() {
				self.view = View::Browser;
			}
		}

	}

	fn close_cur_buf(&mut self) {
		if let Some(id) = self.cur_buf {
			self.close_buf(id);
		}
	}

	fn new_buf(&mut self, mut b: impl Buffer) {

		let id = self.last_buf_id;

		b.set_active(true);

		self.buffers.insert(id, Box::new(b));
		self.last_buf_id += 1;
		self.to_buf(id);

	}

	fn open(&mut self, d: &mut Ctx, path: impl AsRef<Path>) -> Result<()> {

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
				=> return Ok(self.new_buf(ImageViewer::new(path)?)),

				"glb"
				| "obj"
				=> return Ok(self.new_buf(ModelViewer::new(d, path)?)),

				"mp3"
				| "ogg"
				| "wav"
				=> return Ok(self.new_buf(MusicPlayer::new(path)?)),

				"blend"
				| "ase"
				| "pdf"
				| "app"
				| "mp4"
				=> return sysopen(path),

				_ => {},

			}

		}

		self.new_buf(TextEditor::new(path));

		return Ok(());

	}

	fn new_file(&mut self, fname: &str) {

		let path = self.browser.path().join(fname);

		for (id, buf) in &self.buffers {
			if Some(path.as_ref()) == buf.path() {
				self.to_buf(*id);
				return;
			}
		}

		self.new_buf(TextEditor::new(self.browser.path().join(fname)));

	}

	fn to_bookmark(&mut self, n: ID) {
		if let Some(path) = self.bookmarks.get(&n) {
			self.browser.cd(&path.clone());
			self.view = View::Browser;
		}
	}

}

impl State for App {

	fn init(d: &mut Ctx) -> Result<Self> {

		let path = std::env::current_dir()
			.map_err(|_| format!("failed to get current path"))?;

		let session = Session::load().unwrap_or_else(|_| {
			return Session {
				path: path.clone(),
				bufs: vec![],
			};
		});

		let conf = Conf::load().unwrap_or_default();

		let mut app = Self {
			bookmarks: conf.bookmarks,
			browser: FileBrowser::new(session.path)?,
			term: Term::new(),
			view: View::Browser,
			buffers: bmap![],
			last_buf_id: 0,
			cur_buf: None,
			bufbar_offset: 0.0,
			log: vecd![],
		};

		for path in session.bufs {
			app.open(d, path)?;
		}

		app.view = View::Browser;

		return Ok(app);

	}

	fn event(&mut self, d: &mut Ctx, e: &input::Event) -> Result<()> {

		let kmods = d.window.key_mods();

		match self.view {
			View::Buffer => {
				if let Some(buf) = self.cur_buf_mut() {
					buf.event(d, e)?;
				}
				match e {
					Event::KeyPress(k) => {
						match k {
							Key::W if kmods.alt => self.close_cur_buf(),
							_ => {},
						}
					},
					_ => {},
				}
			},
			View::Browser => {
				match e {
					Event::KeyPress(k) => {
						match k {
							Key::Enter => {
								if let Some(file) = self.browser.enter() {
									self.open(d, file)?;
								}
							},
							_ => {},
						}
					},
					_ => {},
				}
				self.browser.event(d, e)?;
			},
			View::Term => {
				self.term.event(d, e)?;
			},
		}

		let path = self.browser.path().to_path_buf();

		match e {
			Event::KeyPress(k) => {
				match k {
					Key::Key1 if kmods.alt => self.to_buf_n(0),
					Key::Key2 if kmods.alt => self.to_buf_n(1),
					Key::Key3 if kmods.alt => self.to_buf_n(2),
					Key::Key4 if kmods.alt => self.to_buf_n(3),
					Key::Key5 if kmods.alt => self.to_buf_n(4),
					Key::Key6 if kmods.alt => self.to_buf_n(5),
					Key::Key7 if kmods.alt => self.to_buf_n(6),
					Key::Key8 if kmods.alt => self.to_buf_n(7),
					Key::Key9 if kmods.alt => self.to_buf_n(8),
					Key::Q if kmods.alt => self.to_prev_buf(),
					Key::E if kmods.alt => self.to_next_buf(),
					Key::F1 => self.to_bookmark(0),
					Key::F2 => self.to_bookmark(1),
					Key::F3 => self.to_bookmark(2),
					Key::F4 => self.to_bookmark(3),
					Key::F5 => self.to_bookmark(4),
					Key::F6 => self.to_bookmark(5),
					Key::F7 => self.to_bookmark(6),
					Key::F8 => self.to_bookmark(7),
					Key::F9 => self.to_bookmark(8),
					Key::F10 => self.to_bookmark(9),
					Key::Q if kmods.meta => d.window.quit(),
					Key::F if kmods.meta => d.window.toggle_fullscreen(),
// 					Key::Backquote => {
// 						self.view = match self.view {
// 							View::Term => View::Browser,
// 							_ => View::Term,
// 						};
// 					},
					Key::Tab => {
						self.view = match self.view {
							View::Buffer => {
								if let Some(buf) = self.cur_buf() {
									let path = buf.path().map(|p| p.to_path_buf());
									if buf.busy() {
										View::Buffer
									} else {
										self.browser.refresh();
										if let Some(path) = path {
											self.browser.select(path);
										}
										View::Browser
									}
								} else {
									View::Browser
								}
							},
							View::Browser => {
								if self.cur_buf().is_some() {
									View::Buffer
								} else {
									View::Browser
								}
							},
							View::Term => View::Term,
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

		match self.view {
			View::Buffer => {
				if let Some(buf) = self.cur_buf_mut() {
					buf.update(d)?;
				}
			},
			View::Browser => self.browser.update(d)?,
			View::Term => self.term.update(d)?,
		}

		self.log.extend(mem::replace(self.browser.log(), vec![]));

		for b in self.buffers.values_mut() {
			if let Some(log) = b.log() {
				self.log.extend(mem::replace(log, vec![]));
			}
		}

		while self.log.len() > LOG_SIZE {
			self.log.pop_front();
		}

		self.log.retain(|l| l.age() < Duration::from_secs_f32(LOG_LIFE));

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
				&shapes::text(&format!("{}", display_path(self.cur_path())))
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

							let title = if b.modified() {
								format!("{} [~]", b.title())
							} else {
								b.title()
							};

							gfx.draw_t(
								mat4!()
									.t2(BUFBAR_PADDING * vec2!(1, -1))
									,
								&shapes::text(&title)
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
				View::Term => {
					self.term.set_view_size(gw, gh - y);
					self.term.draw(gfx)?;
				},
			}

			return Ok(());

		})?;

		return Ok(());

	}

	fn quit(&mut self, _: &mut Ctx) -> Result<()> {

		Session {
			path: self.browser
				.path()
				.to_path_buf(),
			bufs: self.buffers
				.values()
				.map(|b| b.path().map(Path::to_path_buf))
				.flatten()
				.collect(),
		}.save()?;

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
		.title("space55")
		.size(960, 640)
		.resizable(true)
		.run::<App>() {
		elog!("{}", e);
	}

}

