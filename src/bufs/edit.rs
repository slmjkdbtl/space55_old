// wengwengweng

use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;

use crate::*;
use kit::textedit::*;

use rayon::prelude::*;
use once_cell::sync::Lazy;
use syntect::parsing::SyntaxSet;
use syntect::parsing::SyntaxReference;
use syntect::parsing::ScopeStack;
use syntect::highlighting::ThemeSet;
use syntect::highlighting::Theme;
use syntect::highlighting::Highlighter;
use syntect::highlighting::HighlightIterator;

const LINE_SPACING: f32 = 3.0;
const FONT_SIZE: f32 = 12.0;
const LINE_HEIGHT: f32 = FONT_SIZE + LINE_SPACING;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
	return syntect::dumps::from_binary(include_bytes!("syntaxset.pack"));
});

static WRAP_CHARS: Lazy<HashMap<char, char>> = Lazy::new(|| {
	return hmap![
		'(' => ')',
		'[' => ']',
		'{' => '}',
		'"' => '"',
		'\'' => '\'',
	];
});

static SCOPE_CHARS: Lazy<HashMap<char, char>> = Lazy::new(|| {
	return hmap![
		'(' => ')',
		'[' => ']',
		'{' => '}',
	];
});

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
	Normal,
	Insert,
	Select,
	Command,
}

#[derive(Clone, Copy, Debug)]
enum Command {
	Insert(char),
	MoveUp,
	MoveDown,
	MoveLeft,
	MoveRight,
	MovePrevWord,
	MoveNextWord,
	MoveLineStart,
	MoveLineEnd,
	DelLine,
	DelWord,
	Del,
	Undo,
	Redo,
	BreakLine,
}

pub struct TextEditor {
	buf: TextArea,
	path: PathBuf,
	render_lines: Vec<RenderedLine>,
	mode: Mode,
	scroll_off: f32,
	scroll_remainder: f32,
	view_size: Option<(f32, f32)>,
	recording: bool,
	record: Vec<Command>,
	highlight_ctx: Option<HighlightCtx>,
	cmd_bar: Input,
	search_query: Option<String>,
}

#[derive(Clone)]
struct HighlightCtx {
	theme: Theme,
	syntax: SyntaxReference,
	states: Vec<HighlightState>,
}

#[derive(Clone)]
struct HighlightState {
	parse: syntect::parsing::ParseState,
	highlight: syntect::highlighting::HighlightState,
}

impl HighlightState {

	pub fn new(syntax: &SyntaxReference, theme: &Theme) -> Self {

		let highlighter = Highlighter::new(theme);

		return Self {
			parse: syntect::parsing::ParseState::new(&syntax),
			highlight: syntect::highlighting::HighlightState::new(&highlighter, ScopeStack::new()),
		};

	}

}

type RenderedLine = Vec<TextChunk>;

#[derive(Clone)]
struct TextChunk {
	text: String,
	color: Color,
}

impl TextEditor {

	pub fn new(path: impl AsRef<Path>) -> Self {

		let path = path.as_ref();
		let mut buf = TextArea::new();

		if let Ok(content) = fs::read_str(&path) {
			buf.set_content(&content);
		}

		let syntax = SYNTAX_SET
			.find_syntax_for_file(path)
			.ok()
			.flatten()
			.cloned();

		let theme = ThemeSet::load_from_reader(&mut Cursor::new(&include_str!("themes/dracula.tmTheme")[..])).ok();

		let mut hi_ctx = match (syntax, theme) {
			(Some(s), Some(t)) => {
				Some(HighlightCtx {
					states: vec![],
					syntax: s,
					theme: t,
				})
			},
			_ => None
		};

		let lines = buf.lines();

		let render_lines = if let Some(ctx) = &mut hi_ctx {

			let mut rlines = Vec::with_capacity(lines.len());
			let highlighter = Highlighter::new(&ctx.theme);
			let mut state = HighlightState::new(&ctx.syntax, &ctx.theme);

			for l in lines {

				let ops = state.parse.parse_line(l, &SYNTAX_SET);
				let iter = HighlightIterator::new(&mut state.highlight, &ops, l, &highlighter);

				rlines.push(iter.map(|(s, text)| {
					return TextChunk {
						text: text.to_string(),
						color: rgba!(
							s.foreground.r as f32 / 255.0,
							s.foreground.g as f32 / 255.0,
							s.foreground.b as f32 / 255.0,
							s.foreground.a as f32 / 255.0,
						),
					};
				}).collect::<Vec<TextChunk>>());

			}

			rlines

		} else {

			lines.into_par_iter().map(|l| {
				return vec![TextChunk {
					color: rgba!(1),
					text: String::from(l),
				}];
			}).collect()

		};

		return Self {
			buf: buf,
			path: path.to_path_buf(),
			render_lines: render_lines,
			mode: Mode::Normal,
			scroll_off: 0.0,
			scroll_remainder: 0.0,
			view_size: None,
			recording: false,
			record: vec![],
			highlight_ctx: hi_ctx,
			cmd_bar: Input::new(),
			search_query: None,
		};

	}

	fn save(&mut self) -> Result<()> {
		self.buf.clear_modified();
		return std::fs::write(&self.path, self.buf.content())
			.map_err(|_| format!("failed to write to {}", self.path.display()));
	}

	fn exec(&mut self, cmd: Command) {

		if self.recording {
			self.record.push(cmd.clone());
		}

		match cmd {
			Command::Insert(ch) => self.buf.insert(ch),
			Command::MoveUp => self.buf.move_up(),
			Command::MoveDown => self.buf.move_down(),
			Command::MoveLeft => self.buf.move_left(),
			Command::MoveRight => self.buf.move_right(),
			Command::MovePrevWord => self.buf.move_prev_word(),
			Command::MoveNextWord => self.buf.move_next_word(),
			Command::MoveLineStart => self.buf.move_line_start(),
			Command::MoveLineEnd => self.buf.move_line_end(),
			Command::DelLine => self.buf.del_line(),
			Command::DelWord => self.buf.del_word(),
			Command::Del => self.buf.del(),
			Command::Undo => self.buf.undo(),
			Command::Redo => self.buf.redo(),
			Command::BreakLine => self.buf.break_line(),
		}

	}

	fn highlight_all(&mut self) {

		let lines = self.buf.lines();

		self.render_lines = if let Some(ctx) = &mut self.highlight_ctx {

			let mut rlines = Vec::with_capacity(lines.len());
			let highlighter = Highlighter::new(&ctx.theme);
			let mut state = HighlightState::new(&ctx.syntax, &ctx.theme);

			for l in lines {

				let ops = state.parse.parse_line(l, &SYNTAX_SET);
				let iter = HighlightIterator::new(&mut state.highlight, &ops, l, &highlighter);

				rlines.push(iter.map(|(s, text)| {
					return TextChunk {
						text: text.to_string(),
						color: rgba!(
							s.foreground.r as f32 / 255.0,
							s.foreground.g as f32 / 255.0,
							s.foreground.b as f32 / 255.0,
							s.foreground.a as f32 / 255.0,
						),
					};
				}).collect::<Vec<TextChunk>>());

				ctx.states.push(state.clone());

			}

			rlines

		} else {

			lines.into_par_iter().map(|l| {
				return vec![TextChunk {
					color: rgba!(1),
					text: String::from(l),
				}];
			}).collect()

		};

	}

}

impl Buffer for TextEditor {

	fn path(&self) -> Option<&Path> {
		return Some(&self.path);
	}

	fn modified(&self) -> bool {
		return self.buf.modified();
	}

	fn set_view_size(&mut self, w: f32, h: f32) {
		self.view_size = Some((w, h));
	}

	fn busy(&self) -> bool {
		return self.mode == Mode::Insert;
	}

	fn closable(&self) -> bool {
		return !self.buf.modified();
	}

	fn event(&mut self, d: &mut Ctx, e: &input::Event) -> Result<()> {

		let (vw, vh) = self.view_size.unwrap_or((d.gfx.width() as f32, d.gfx.height() as f32));
		let l1 = f32::floor(self.scroll_off / LINE_HEIGHT) as usize;
		let l2 = f32::ceil((self.scroll_off + vh) / LINE_HEIGHT) as usize;
		let kmods = d.window.key_mods();

		match e {

			Event::KeyPress(k) => {

				match self.mode {
					Mode::Normal => {
						match k {
							Key::Enter => self.mode = Mode::Insert,
							Key::W => self.save()?,
							Key::Backslash => {
								if self.recording {
									self.recording = false;
								} else {
									self.recording = true;
									self.record.clear();
								}
							},
							Key::Period if kmods.alt => {
								for i in 0..self.record.len() {
									self.exec(self.record[i]);
								}
								self.highlight_all();
							},
							_ => {},
						}
					},
					Mode::Insert => {
						match k {
							Key::Esc => self.mode = Mode::Normal,
							_ => {},
						}
					},
					Mode::Select => {},
					Mode::Command => {
						match k {
							Key::Esc => self.mode = Mode::Normal,
							Key::Enter => {
								self.search_query = Some(self.cmd_bar.content().to_string());
								self.mode = Mode::Normal;
							},
							_ => {},
						}
					},

				}

			}

			Event::KeyPressRepeat(k) => {

				match self.mode {

					Mode::Normal => {

						match *k {
							Key::K => self.exec(Command::MoveUp),
							Key::J => self.exec(Command::MoveDown),
							Key::H => {
								if kmods.alt {
									self.exec(Command::MovePrevWord);
								} else {
									self.exec(Command::MoveLeft);
								}
							},
							Key::L => {
								if kmods.alt {
									self.exec(Command::MoveNextWord);
								} else {
									self.exec(Command::MoveRight);
								}
							},
							Key::Left => self.exec(Command::MoveLeft),
							Key::Right => self.exec(Command::MoveRight),
							Key::Up => self.exec(Command::MoveUp),
							Key::Down => self.exec(Command::MoveDown),
							Key::D => {
								self.exec(Command::DelLine);
								self.highlight_all();
							},
							Key::U => {
								self.exec(Command::Undo);
								self.highlight_all();
							},
							Key::O => {
								self.exec(Command::Redo);
								self.highlight_all();
							},
							Key::Semicolon if kmods.alt => {
								// TODO: prev search
							},
							Key::Quote if kmods.alt => {
								// TODO: next search
							},
							_ => {},
						}

					},

					Mode::Insert => {

						match k {

							Key::Backspace => {

								if kmods.alt {
									self.exec(Command::DelWord);
									self.highlight_all();
								} else {

									if let Some(cur_char) = self.buf.cur_char() {
										if let Some(_) = WRAP_CHARS.get(&cur_char) {
											self.exec(Command::MoveRight);
											self.exec(Command::Del);
										}
									}

									self.exec(Command::Del);
									self.highlight_all();

								}

							},

							Key::Enter => {

								let line = self.buf.get_line().cloned();
								let cursor = self.buf.cursor();

								self.exec(Command::BreakLine);

								let mut level = 0;

								if let Some(cur_line) = line {

									for ch in cur_line.chars() {
										if ch == '\t' {
											level += 1;
										} else {
											break;
										}
									}

									let mut chars = cur_line
										.chars()
										.skip((cursor.col - 2) as usize);

									if let Some(ch) = chars.next() {
										if let Some(wch) = SCOPE_CHARS.get(&ch) {
											level += 1;
											if Some(*wch) == chars.next() {
												self.exec(Command::BreakLine);
												for _ in 0..level - 1 {
													self.exec(Command::Insert('\t'));
												}
												self.exec(Command::MoveUp);
											}
										}
									}

								}

								for _ in 0..level {
									self.exec(Command::Insert('\t'));
								}

								self.highlight_all();

							},

							Key::Left => self.exec(Command::MoveLeft),
							Key::Right => self.exec(Command::MoveRight),
							Key::Tab => {
								self.exec(Command::Insert('\t'));
								self.highlight_all();
							},
							_ => {},
						}

					},

					Mode::Select => {},

					Mode::Command => {
						match k {
							Key::Backspace if kmods.alt => self.cmd_bar.del_word(),
							Key::Backspace => self.cmd_bar.del(),
							Key::Left => self.cmd_bar.move_left(),
							Key::Right => self.cmd_bar.move_right(),
							_ => {},
						}
					},

				}

			},

			Event::CharInput(ch) => {

				match self.mode {

					Mode::Normal => {

						match ch {
							'<' => {
								self.exec(Command::MoveLineStart);
								self.mode = Mode::Insert;
							},
							'>' => {
								self.exec(Command::MoveLineEnd);
								self.mode = Mode::Insert;
							},
							'?' => {
								self.mode = Mode::Command;
								self.cmd_bar = Input::new();
							},
							_ => {},
						}

					},

					Mode::Insert => {

						self.exec(Command::Insert(*ch));

						if let Some(wch) = WRAP_CHARS.get(ch) {
							self.exec(Command::Insert(*wch));
							self.exec(Command::MoveLeft);
						}

						self.highlight_all();

					},

					Mode::Command => {
						self.cmd_bar.insert(*ch);
					},

					_ => {},

				}

			}

			Event::Wheel(d, _) => {

				if let Mode::Normal = self.mode {

					let y = d.y * 0.1;
					self.scroll_remainder = (y + self.scroll_remainder) % 1.0;
					let y = (y + self.scroll_remainder) as i32;

					for _ in 0..y.abs() {
						if y > 0 {
							self.buf.move_down();
						} else if y < 0 {
							self.buf.move_up();
						}
					}

				}

			},

			_ => {},

		}

		return Ok(());

	}

	fn update(&mut self, d: &mut Ctx) -> Result<()> {

		let (vw, vh) = self.view_size.unwrap_or((d.gfx.width() as f32, d.gfx.height() as f32));
		let height = LINE_HEIGHT * self.buf.cursor().line as f32;

		let y = height - self.scroll_off;

		if y > vh {
			self.scroll_off = height - vh;
		}

		if self.scroll_off > (height - LINE_HEIGHT) {
			self.scroll_off = height - LINE_HEIGHT;
		}

		return Ok(());

	}

	fn draw(&self, gfx: &mut Gfx) -> Result<()> {

		let mut y = LINE_SPACING * 0.5;
		let (vw, vh) = self.view_size.unwrap_or((gfx.width() as f32, gfx.height() as f32));

		// TODO: apply scroll off remainder
		let l1 = f32::floor(self.scroll_off / LINE_HEIGHT) as usize;
		let l2 = f32::ceil((self.scroll_off + vh) / LINE_HEIGHT) as usize;

		let cursor = self.buf.cursor();

		for i in l1..l2 {

			if let Some(chunks) = self.render_lines.get(i) {

				let chunks = chunks
					.iter()
					.map(|c| {
						return shapes::textc(&c.text, c.color);
					})
					.collect::<Vec<shapes::TextChunk>>();

				let ftext = shapes::Text::from_chunks(&chunks)
					.align(gfx::Origin::TopLeft)
					.line_spacing(LINE_SPACING)
					.size(FONT_SIZE)
					.tab_width(4)
					.format(gfx)
					;

				if cursor.line == i as i32 + 1 {

					let color = match self.mode {
						Mode::Normal => rgba!(1),
						Mode::Insert => rgba!(0, 1, 1, 1),
						Mode::Select => rgba!(1),
						Mode::Command => rgba!(1, 1, 1, 0),
					};

					if let Some(pos) = ftext.cursor_pos(cursor.col as usize - 1) {

						let padding = 2.0;

						// draw cursor
						gfx.draw(
							&shapes::rect(
								pos + vec2!(0, -y + padding),
								pos + vec2!(12.0, -y - FONT_SIZE - padding)
							)
								.fill(color)
								,
						)?;

						// draw cursor line
						gfx.draw(
							&shapes::rect(
								vec2!(0, -y + padding),
								vec2!(vw, -y - FONT_SIZE - padding)
							)
								.fill(rgba!(1, 1, 1, 0.1))
								,
						)?;

					}

				}

				gfx.draw_t(
					mat4!()
						.ty(-y)
						,
					&ftext,
				)?;

				if y >= vh {
					break;
				}

				y += LINE_HEIGHT;

			}

		}

		let (m, c) = match self.mode {
			Mode::Normal => ("normal", rgba!(0.5, 1, 1, 1)),
			Mode::Insert => ("insert", rgba!(0.5, 1, 0.5, 1)),
			Mode::Select => ("select", rgba!(1, 0.5, 0.5, 1)),
			Mode::Command => ("command", rgba!(1, 1, 0.5, 1)),
		};

		gfx.draw(
			&shapes::rect(
				vec2!(0, -vh + FONT_SIZE + 4.0),
				vec2!(vw, -vh),
			)
				.fill(c)
		)?;

		gfx.draw_t(
			mat4!()
				.t2(vec2!(2.0, -vh + 2.0))
				,
			&shapes::text(&format!("{}", m.to_uppercase()))
				.align(Origin::BottomLeft)
				.size(FONT_SIZE)
				.color(rgba!(0, 0, 0, 1))
		)?;

// 		if let Mode::Command = self.mode {

// 			gfx.draw(
// 				&shapes::rect(
// 					vec2!(0, -vh + FONT_SIZE),
// 					vec2!(vw, -vh),
// 				)
// 					.fill(rgba!(0, 0, 0, 1))
// 			)?;

// 			let cmd_bar = shapes::text(self.cmd_bar.content())
// 				.align(Origin::BottomLeft)
// 				.size(FONT_SIZE)
// 				.format(gfx);

// 			if let Some(pos) = cmd_bar.cursor_pos(self.cmd_bar.cursor() as usize) {
// 				gfx.draw_t(
// 					mat4!()
// 						.t2(vec2!(0, -vh))
// 						.t2(pos)
// 						,
// 					&shapes::rect(vec2!(0), vec2!(12.0, -FONT_SIZE)),
// 				)?;
// 			}

// 			gfx.draw_t(
// 				mat4!()
// 					.t2(vec2!(0, -vh))
// 					,
// 				&cmd_bar
// 			)?;

// 		}

		return Ok(());

	}

}

