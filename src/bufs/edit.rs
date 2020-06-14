// wengwengweng

use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;

use crate::*;
use kit::textedit::TextArea;

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

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
	Normal,
	Insert,
}

pub struct TextEditor {
	buf: TextArea,
	path: PathBuf,
	render_lines: Vec<RenderedLine>,
	mode: Mode,
	scroll_off: f32,
	scroll_remainder: f32,
	view_size: Option<(f32, f32)>,
	highlight_ctx: Option<HighlightCtx>,
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
			highlight_ctx: hi_ctx,
		};

	}

	pub fn save(&self) -> Result<()> {
		return std::fs::write(&self.path, self.buf.content())
			.map_err(|_| format!("failed to write to {}", self.path.display()));
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

	fn set_view_size(&mut self, w: f32, h: f32) {
		self.view_size = Some((w, h));
	}

	fn busy(&self) -> bool {
		return self.mode == Mode::Insert;
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
							_ => {},
						}
					},
					Mode::Insert => {
						match k {
							Key::Esc => self.mode = Mode::Normal,
							_ => {},
						}
					},
				}

			}

			Event::KeyPressRepeat(k) => {

				match self.mode {
					Mode::Normal => {
						match *k {
							Key::K => self.buf.move_up(),
							Key::J => self.buf.move_down(),
							Key::H => {
								if kmods.alt {
									self.buf.move_prev_word();
								} else {
									self.buf.move_left();
								}
							},
							Key::L => {
								if kmods.alt {
									self.buf.move_next_word();
								} else {
									self.buf.move_right();
								}
							},
							Key::Left => self.buf.move_left(),
							Key::Right => self.buf.move_right(),
							Key::Up => self.buf.move_up(),
							Key::Down => self.buf.move_down(),
							Key::D => {
								self.buf.del_line();
								self.highlight_all();
							},
							Key::U => {
								self.buf.undo();
								self.highlight_all();
							},
							Key::O => {
								self.buf.redo();
								self.highlight_all();
							},
							_ => {},
						}
					},
					Mode::Insert => {
						match k {
							Key::Backspace => {
								if kmods.alt {
									self.buf.del_word();
									self.highlight_all();
								} else {
									self.buf.del();
									self.highlight_all();
								}
							},
							Key::Enter => {
								self.buf.break_line();
								self.highlight_all();
							},
							Key::Left => self.buf.move_left(),
							Key::Right => self.buf.move_right(),
							Key::Up => self.buf.move_up(),
							Key::Down => self.buf.move_down(),
							Key::Tab => {
								self.buf.insert('\t');
								self.highlight_all();
							},
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
								self.buf.move_line_start();
								self.mode = Mode::Insert;
							},
							'>' => {
								self.buf.move_line_end();
								self.mode = Mode::Insert;
							},
							_ => {},
						}
					}
					Mode::Insert => {
						self.buf.insert(*ch);
						self.highlight_all();
					},
				}
			}

			Event::Wheel(d, _) => {

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
					};

					if let Some(pos) = ftext.cursor_pos(cursor.col as usize - 1) {

						let padding = 2.0;

						// draw cursor
						gfx.draw(
							&shapes::rect(
								pos + vec2!(0, -y + padding),
								pos + vec2!(3, -y - FONT_SIZE - padding)
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

		return Ok(());

	}

}

