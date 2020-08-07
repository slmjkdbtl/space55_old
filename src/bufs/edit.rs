// wengwengweng

// TODO: clean up

use std::fmt;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashSet;
use std::collections::HashMap;

use crate::*;
use kit::textinput::*;

use rayon::prelude::*;
use once_cell::sync::Lazy;
use syntect::parsing::SyntaxSet;
use syntect::parsing::SyntaxReference;
use syntect::parsing::ScopeStack;
use syntect::highlighting::ThemeSet;
use syntect::highlighting::Theme;
use syntect::highlighting::Highlighter;
use syntect::highlighting::HighlightIterator;

type Line = i32;
type Col = i32;

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

static BREAK_CHARS: Lazy<HashSet<char>> = Lazy::new(|| {
	return hset![' ', ',', '.', ';', ':', '"', '(', ')', '{', '}', '[', ']', '<', '>', '_', '-', '@', '/', '\\', '\'', '\t' ];
});

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cursor {
	pub line: Line,
	pub col: Col,
}

impl Cursor {
	fn new(l: Line, c: Col) -> Self {
		return Self {
			line: l,
			col: c,
		};
	}
}

impl fmt::Display for Cursor {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		return write!(f, "{}:{}", self.line, self.col);
	}
}

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
	MoveTo(Cursor),
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

#[derive(Debug, Clone, PartialEq)]
struct State {
	lines: Vec<String>,
	cursor: Cursor,
	modified: bool,
}

pub struct TextEditor {
	lines: Vec<String>,
	cursor: Cursor,
	modified: bool,
	undo_stack: Vec<State>,
	redo_stack: Vec<State>,
	path: PathBuf,
	rendered_lines: Vec<RenderedLine>,
	mode: Mode,
	scroll_off: f32,
	scroll_remainder: f32,
	view_size: Option<(f32, f32)>,
	recording: bool,
	record: Vec<Command>,
	highlight_ctx: Option<HighlightCtx>,
	cmd_bar: Input,
	search_pattern: Option<regex::Regex>,
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

	fn new(syntax: &SyntaxReference, theme: &Theme) -> Self {

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

		let content = std::fs::read_to_string(&path)
			.unwrap_or(String::new());

		let mut lines = content
			.split('\n')
			.map(|s| s.to_string())
			.collect::<Vec<String>>();

		lines.pop();

		let syntax = SYNTAX_SET
			.find_syntax_for_file(path)
			.ok()
			.flatten()
			.cloned();

		let theme = ThemeSet::load_from_reader(&mut io::Cursor::new(&include_str!("themes/dracula.tmTheme")[..])).ok();

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

		let rendered_lines = if let Some(ctx) = &mut hi_ctx {

			let mut rlines = Vec::with_capacity(lines.len());
			let highlighter = Highlighter::new(&ctx.theme);
			let mut state = HighlightState::new(&ctx.syntax, &ctx.theme);

			for l in &lines {

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

			lines.par_iter().map(|l| {
				return vec![TextChunk {
					color: rgba!(1),
					text: String::from(l),
				}];
			}).collect()

		};

		return Self {
			lines: lines,
			cursor: Cursor::new(1, 1),
			undo_stack: vec![],
			redo_stack: vec![],
			modified: false,
			path: path.to_path_buf(),
			rendered_lines: rendered_lines,
			mode: Mode::Normal,
			scroll_off: 0.0,
			scroll_remainder: 0.0,
			view_size: None,
			recording: false,
			record: vec![],
			highlight_ctx: hi_ctx,
			cmd_bar: Input::new(),
			search_pattern: None,
		};

	}

	fn content(&self) -> String {
		return self.lines.join("\n");
	}

	fn get_line_at(&self, ln: Line) -> Option<&String> {
		if ln > 0 {
			return self.lines.get(ln as usize - 1);
		}
		return None;
	}

	fn cur_line(&self) -> Option<&String> {
		return self.get_line_at(self.cursor.line);
	}

	fn set_line_at(&mut self, ln: Line, content: &str) {

		if self.get_line_at(ln).is_some() {

			// TODO: clean logic
			if !self.modified {
				self.push_undo();
				self.redo_stack.clear();
				self.modified = true;
			}

			self.lines.get_mut(ln as usize - 1).map(|s| *s = String::from(content));

		}

	}

	fn set_line(&mut self, content: &str) {
		self.set_line_at(self.cursor.line, content);
	}

	fn insert_str_at(&mut self, mut pos: Cursor, text: &str) -> Cursor {

		if let Some(mut line) = self.get_line_at(pos.line).map(Clone::clone) {

			line.insert_str(pos.col as usize - 1, text);
			self.push_undo();
			self.set_line_at(pos.line, &line);
			pos.col += text.len() as Col;

			return self.clamp_cursor(pos);

		}

		return pos;

	}

	fn insert_str(&mut self, text: &str) {
		self.cursor = self.insert_str_at(self.cursor, text);
	}

	fn insert_at(&mut self, mut pos: Cursor, ch: char) -> Cursor {

		if !ch.is_ascii() {
			return pos;
		}

		if let Some(mut line) = self.get_line_at(pos.line).map(Clone::clone) {

			line.insert(pos.col as usize - 1, ch);

			if BREAK_CHARS.contains(&ch) {
				self.push_undo();
			}

			self.set_line_at(pos.line, &line);
			pos.col += 1;

			return self.clamp_cursor(pos);

		}

		return pos;

	}

	fn insert(&mut self, ch: char) {
		self.cursor = self.insert_at(self.cursor, ch);
	}

	fn del_line_at(&mut self, ln: Line) -> Line {

		if ln as usize <= self.lines.len() {

			self.push_undo();

			if !self.modified {
				self.redo_stack.clear();
				self.modified = true;
			}

			self.lines.remove(ln as usize - 1);

			if self.lines.is_empty() {
				self.lines = vec![String::new()];
			}

		}

		return ln.max(1).min(self.lines.len() as Line);

	}

	fn del_line(&mut self) {
		self.cursor.line = self.del_line_at(self.cursor.line);
	}

	fn char_at(&self, pos: Cursor) -> Option<char> {
		return self.get_line_at(pos.line)?.chars().nth(pos.col as usize - 1);
	}

	fn cur_char(&self) -> Option<char> {
		return self.char_at(self.cursor);
	}

	fn insert_line_at(&mut self, line: Line) {
		self.push_undo();
		self.lines.insert(line as usize, String::new());
	}

	fn insert_line(&mut self) {
		self.insert_line_at(self.cursor.line);
	}

	fn break_line_at(&mut self, mut pos: Cursor) -> Cursor {

		if let Some(line) = self.get_line_at(pos.line).map(Clone::clone) {

			let before = String::from(&line[0..pos.col as usize - 1]);
			let after = String::from(&line[pos.col as usize - 1..line.len()]);

			self.push_undo();

			if !self.modified {
				self.redo_stack.clear();
				self.modified = true;
			}

			self.lines.insert(pos.line as usize, String::new());
			self.set_line_at(pos.line, &before);
			self.set_line_at(pos.line + 1, &after);
			pos.line += 1;
			pos.col = 1;

			return self.clamp_cursor(pos);

		}

		return pos;

	}

	fn break_line(&mut self) {
		self.cursor = self.break_line_at(self.cursor);
	}

	fn del_at(&mut self, mut pos: Cursor) -> Cursor {

		if let Some(mut line) = self.get_line_at(pos.line).map(Clone::clone) {

			let before = &line[0..pos.col as usize - 1];

			if before.is_empty() {

				if let Some(mut prev_line) = self.get_line_at(pos.line - 1).map(Clone::clone) {

					let col = prev_line.len() as Col + 1;

					prev_line.push_str(&line);
					self.del_line_at(pos.line);
					self.set_line_at(pos.line - 1, &prev_line);
					pos.line -= 1;
					pos.col = col;

				}

			} else {

				line.remove(pos.col as usize - 2);
				self.set_line_at(pos.line, &line);
				pos.col -= 1;

			}

			return pos;

		}

		return pos;

	}

	fn del(&mut self) {
		self.cursor = self.del_at(self.cursor);
	}

	fn del_word_at(&mut self, mut pos: Cursor) -> Cursor {

		if let Some(line) = self.get_line_at(pos.line).map(Clone::clone) {

			let before = &line[0..pos.col as usize - 1];

			if before.is_empty() {

				if let Some(mut prev_line) = self.get_line_at(pos.line - 1).map(Clone::clone) {

					let col = prev_line.len() as Col + 1;

					prev_line.push_str(&line);
					self.del_line_at(pos.line);
					self.set_line_at(pos.line - 1, &prev_line);
					pos.line -= 1;
					pos.col = col;

				}

			} else if let Some(prev_pos) = self.prev_word_at(pos) {
				return self.del_range((prev_pos, Cursor {
					col: pos.col - 1,
					.. pos
				}));
			}

		}

		return pos;

	}

	fn del_word(&mut self) {
		let pos = self.del_word_at(self.cursor);
		self.move_to(pos);
	}

	// TODO: multiline
	fn del_range(&mut self, r: (Cursor, Cursor)) -> Cursor {

		let (start, end) = r;

		if start.line == end.line {

			if let Some(line) = self.get_line_at(start.line) {

				let mut line = line.clone();
				let start_col = (start.col - 1).max(0).min(line.len() as i32);
				let end_col = end.col.max(0).min(line.len() as i32);

				self.push_undo();
				line.replace_range(start_col as usize..end_col as usize, "");
				self.set_line_at(start.line, &line);

				return start;

			}

		}

		return self.cursor;

	}

	fn clamp_cursor(&self, mut pos: Cursor) -> Cursor {

		if pos.col < 1 {
			pos.col = 1;
		}

		if pos.line < 1 {
			pos.line = 1;
		}

		if pos.line > self.lines.len() as i32 {
			pos.line = self.lines.len() as i32;
		}

		if let Some(line) = self.get_line_at(pos.line) {

			let len = line.len() as Col + 1;

			if pos.col > len {
				pos.col = len;
			}

		}

		return pos;

	}

	fn move_to(&mut self, pos: Cursor) {
		self.cursor = self.clamp_cursor(pos);
	}

	fn move_left(&mut self) {
		self.move_to(Cursor {
			col: self.cursor.col - 1,
			.. self.cursor
		});
	}

	fn move_right(&mut self) {
		self.move_to(Cursor {
			col: self.cursor.col + 1,
			.. self.cursor
		});
	}

	fn move_up(&mut self) {
		self.move_to(Cursor {
			line: self.cursor.line - 1,
			.. self.cursor
		});
	}

	fn move_down(&mut self) {
		self.move_to(Cursor {
			line: self.cursor.line + 1,
			.. self.cursor
		});
	}

	fn move_prev_word(&mut self) {
		if let Some(pos) = self.prev_word() {
			self.move_to(pos);
		}
	}

	fn move_next_word(&mut self) {
		if let Some(pos) = self.next_word() {
			self.move_to(pos);
		}
	}

	fn next_word_at(&self, pos: Cursor) -> Option<Cursor> {

		let line = self.get_line_at(pos.line)?;

		if pos.col < line.len() as Col {

			for (i, ch) in line[pos.col as usize..].char_indices() {

				if BREAK_CHARS.contains(&ch) {
					return Some(Cursor {
						col: pos.col + i as Col + 1 as Col,
						.. pos
					});
				}

			}

			return Some(Cursor {
				col: line.len() as Col + 1,
				.. pos
			});

		}

		return None;

	}

	fn next_word(&self) -> Option<Cursor> {
		return self.next_word_at(self.cursor);
	}

	fn prev_word_at(&self, pos: Cursor) -> Option<Cursor> {

		let line = self.get_line_at(pos.line)?;

		if pos.col <= line.len() as Col + 1 {

			let end = (pos.col - 2).max(0).min(line.len() as i32);

			for (i, ch) in line[..end as usize].char_indices().rev() {

				if BREAK_CHARS.contains(&ch) {
					return Some(Cursor {
						col: i as Col + 2,
						.. pos
					});
				}

			}

			return Some(Cursor {
				col: 1,
				.. pos
			});

		}

		return None;

	}

	fn prev_word(&self) -> Option<Cursor> {
		return self.prev_word_at(self.cursor);
	}

	fn get_state(&self) -> State {
		return State {
			lines: self.lines.clone(),
			cursor: self.cursor.clone(),
			modified: self.modified,
		};
	}

	fn set_state(&mut self, state: State) {
		self.lines = state.lines;
		self.modified = state.modified;
		self.move_to(state.cursor);
	}

	fn push_undo(&mut self) {

		let state = self.get_state();

		if self.undo_stack.last() == Some(&state) {
			return;
		}

		self.undo_stack.push(state);

	}

	fn push_redo(&mut self) {
		self.redo_stack.push(self.get_state());
	}

	fn undo(&mut self) {
		if let Some(state) = self.undo_stack.pop() {
			self.push_redo();
			self.set_state(state);
		}
	}

	fn redo(&mut self) {
		if let Some(state) = self.redo_stack.pop() {
			self.push_undo();
			self.set_state(state);
		}
	}

	fn line_start_at(&self, mut pos: Cursor) -> Cursor {

		if let Some(line) = self.get_line_at(pos.line) {

			let mut index = 0;

			for (i, ch) in line.chars().enumerate() {
				if ch != '\t' && ch != ' ' {
					index = i;
					break;
				} else if i == line.len() - 1 {
					index = i + 1;
				}
			}

			pos.col = index as Col + 1;

			return self.clamp_cursor(pos);

		}

		return pos;

	}

	fn move_line_start(&mut self) {
		self.cursor = self.line_start_at(self.cursor);
	}

	fn line_end_at(&self, mut pos: Cursor) -> Cursor {

		if let Some(line) = self.get_line_at(pos.line) {
			pos.col = line.len() as Col + 1;
			return self.clamp_cursor(pos);
		}

		return pos;

	}

	fn move_line_end(&mut self) {
		self.cursor = self.line_end_at(self.cursor);
	}

	fn clear_modified(&mut self) {
		for s in &mut self.undo_stack {
			s.modified = true;
		}
		self.modified = false;
	}

	fn save(&mut self) -> Result<()> {
		self.trim_all();
		self.clear_modified();
		return std::fs::write(&self.path, self.content())
			.map_err(|_| format!("failed to write to {}", self.path.display()));
	}

	fn exec(&mut self, cmd: Command) {

		if self.recording {
			self.record.push(cmd.clone());
		}

		match cmd {
			Command::Insert(ch) => self.insert(ch),
			Command::MoveTo(c) => self.move_to(c),
			Command::MoveUp => self.move_up(),
			Command::MoveDown => self.move_down(),
			Command::MoveLeft => self.move_left(),
			Command::MoveRight => self.move_right(),
			Command::MovePrevWord => self.move_prev_word(),
			Command::MoveNextWord => self.move_next_word(),
			Command::MoveLineStart => self.move_line_start(),
			Command::MoveLineEnd => self.move_line_end(),
			Command::DelLine => self.del_line(),
			Command::DelWord => self.del_word(),
			Command::Del => self.del(),
			Command::Undo => self.undo(),
			Command::Redo => self.redo(),
			Command::BreakLine => self.break_line(),
		}

	}

	fn highlight_all(&mut self) {

		self.rendered_lines = if let Some(ctx) = &mut self.highlight_ctx {

			let mut rlines = Vec::with_capacity(self.lines.len());
			let highlighter = Highlighter::new(&ctx.theme);
			let mut state = HighlightState::new(&ctx.syntax, &ctx.theme);

			for l in &self.lines {

				let ops = state.parse.parse_line(&l, &SYNTAX_SET);
				let iter = HighlightIterator::new(&mut state.highlight, &ops, &l, &highlighter);

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

			self.lines.par_iter().map(|l| {
				return vec![TextChunk {
					color: rgba!(1),
					text: String::from(l),
				}];
			}).collect()

		};

	}

	fn search_backward(&self) -> Option<Cursor> {

		let pat = match &self.search_pattern {
			Some(pat) => pat,
			None => return None,
		};

		for (i, l) in self.lines
			.iter()
			.enumerate()
			.rev()
			.skip(self.lines.len() - self.cursor.line as usize)
		{
			for f in pat.find_iter(l) {
				let col = f.start() as i32 + 1;
				if !(i as i32 + 1 == self.cursor.line && col >= self.cursor.col) {
					return Some(Cursor::new(i as i32 + 1, col));
				}
			}
		}

		return None;

	}

	fn search_forward(&self) -> Option<Cursor> {

		let pat = match &self.search_pattern {
			Some(pat) => pat,
			None => return None,
		};

		for (i, l) in self.lines
			.iter()
			.enumerate()
			.skip(self.cursor.line as usize - 1)
		{
			for f in pat.find_iter(l) {
				let col = f.start() as i32 + 1;
				if !(i as i32 + 1 == self.cursor.line && col <= self.cursor.col) {
					return Some(Cursor::new(i as i32 + 1, col));
				}
			}
		}

		return None;

	}

	fn trim_all(&mut self) {
		for l in &mut self.lines {
			*l = l.trim_end().to_string();
		}
		self.move_to(self.cursor);
	}

	// TODO: support other comments
	fn toggle_comment(&mut self) {

		self.push_undo();

		let ln = self.cursor.line;

		if let Some(line) = self.get_line_at(ln) {
			if line.starts_with("// ") {
				self.set_line_at(ln, &line[3..].to_string());
			} else {
				self.set_line_at(ln, &format!("// {}", line));
			}
		}

		self.highlight_all();

	}

}

impl Buffer for TextEditor {

	fn path(&self) -> Option<&Path> {
		return Some(&self.path);
	}

	fn modified(&self) -> bool {
		return self.modified;
	}

	fn set_view_size(&mut self, w: f32, h: f32) {
		self.view_size = Some((w, h));
	}

	fn busy(&self) -> bool {
		return self.mode == Mode::Insert;
	}

	fn closable(&self) -> bool {
		return !self.modified;
	}

	fn event(&mut self, d: &mut Ctx, e: &input::Event) -> Result<()> {

		let kmods = d.window.key_mods();

		match e {

			Event::KeyPress(k) => {

				match self.mode {
					Mode::Normal => {
						match k {
							Key::Enter if kmods.alt => {
								self.insert_line();
								self.highlight_all();
							}
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
								self.search_pattern = regex::Regex::new(self.cmd_bar.content()).ok();
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
								if let Some(pos) = self.search_backward() {
									self.exec(Command::MoveTo(pos));
								}
							},
							Key::Quote if kmods.alt => {
								if let Some(pos) = self.search_forward() {
									self.exec(Command::MoveTo(pos));
								}
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

									if let Some(cur_char) = self.cur_char() {
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

								let line = self.cur_line().cloned();
								let cursor = self.cursor;

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
							'/' => self.toggle_comment(),
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
							self.move_down();
						} else if y < 0 {
							self.move_up();
						}
					}

				}

			},

			_ => {},

		}

		return Ok(());

	}

	fn update(&mut self, d: &mut Ctx) -> Result<()> {

		let (_, vh) = self.view_size.unwrap_or((d.gfx.width() as f32, d.gfx.height() as f32));
		let th = vh - FONT_SIZE - LINE_SPACING * 2.0;
		let height = LINE_HEIGHT * self.cursor.line as f32;

		let y = height - self.scroll_off;

		if y > th {
			self.scroll_off = height - th;
		}

		if self.scroll_off > (height - LINE_HEIGHT) {
			self.scroll_off = height - LINE_HEIGHT;
		}

		return Ok(());

	}

	fn draw(&self, gfx: &mut Gfx) -> Result<()> {

		let mut y = LINE_SPACING * 0.5;
		let (vw, vh) = self.view_size.unwrap_or((gfx.width() as f32, gfx.height() as f32));
		let th = vh - FONT_SIZE - LINE_SPACING * 2.0;

		// TODO: apply scroll off remainder
		let l1 = f32::floor(self.scroll_off / LINE_HEIGHT) as usize;
		let l2 = f32::ceil((self.scroll_off + th) / LINE_HEIGHT) as usize;

		let cursor = self.cursor;

		for i in l1..l2 {

			if let Some(chunks) = self.rendered_lines.get(i) {

				let chunks = chunks
					.iter()
					.map(|c| {
						return shapes::TextChunk::colored(&c.text, c.color);
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

				if y >= th {
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
				vec2!(0, -vh + FONT_SIZE + LINE_SPACING * 2.0),
				vec2!(vw, -vh),
			)
				.fill(c)
		)?;

		gfx.draw_t(
			mat4!()
				.t2(vec2!(LINE_SPACING, -vh + LINE_SPACING))
				,
			&shapes::text(&format!("{}", m.to_uppercase()))
				.align(Origin::BottomLeft)
				.size(FONT_SIZE)
				.color(rgba!(0, 0, 0, 1))
		)?;

		gfx.draw_t(
			mat4!()
				.t2(vec2!(vw - LINE_SPACING, -vh + LINE_SPACING))
				,
			&shapes::text(&format!("{}", self.cursor))
				.align(Origin::BottomRight)
				.size(FONT_SIZE)
				.color(rgba!(0, 0, 0, 1))
		)?;

		if let Mode::Command = self.mode {

			gfx.draw(
				&shapes::rect(
					vec2!(0, -vh + FONT_SIZE + 4.0 + FONT_SIZE),
					vec2!(vw, -vh + FONT_SIZE + 4.0),
				)
					.fill(rgba!(0, 0, 0, 1))
			)?;

			let cmd_bar = shapes::text(self.cmd_bar.content())
				.align(Origin::BottomLeft)
				.size(FONT_SIZE)
				.format(gfx);

			if let Some(pos) = cmd_bar.cursor_pos(self.cmd_bar.cursor() as usize) {
				gfx.draw_t(
					mat4!()
						.t2(vec2!(0, -vh + FONT_SIZE + LINE_SPACING * 2.0))
						.t2(pos)
						,
					&shapes::rect(vec2!(0), vec2!(12.0, FONT_SIZE)),
				)?;
			}

			gfx.draw_t(
				mat4!()
					.t2(vec2!(0, -vh + FONT_SIZE + LINE_SPACING * 2.0))
					,
				&cmd_bar
			)?;

		}

		return Ok(());

	}

}

