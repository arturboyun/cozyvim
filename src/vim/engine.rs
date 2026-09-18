//! Модальный интерпретатор vim: разбирает поток нажатий в команды и исполняет
//! их над буфером. Поддерживает подмножество, которое покрывают уроки; всё
//! остальное честно сообщает о себе через `StepResult::Unsupported`.

use std::collections::HashMap;

use crate::core::buffer::Buffer;
use crate::core::keys::Key;

use super::motion::{self, Pos, TextObj};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    VisualLine,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StepResult {
    /// Команда исполнена.
    Consumed,
    /// Ждём продолжения последовательности (`d`, `2`, `f`…).
    Pending,
    /// Команда существует в vim, но тренажёр её пока не умеет.
    Unsupported(String),
    /// Нажатие не значит ничего.
    Ignored,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MotionKind {
    Left,
    Right,
    Down,
    Up,
    WordFwd { big: bool },
    WordBack { big: bool },
    WordEnd { big: bool },
    LineStart,
    FirstNonBlank,
    LineEnd,
    GotoLine,
    FileEnd,
    ParaFwd,
    ParaBack,
    Find { ch: char, forward: bool, till: bool },
    RepeatFind { reverse: bool },
    SearchNext { reverse: bool },
}

impl MotionKind {
    fn linewise(&self) -> bool {
        matches!(
            self,
            MotionKind::Down | MotionKind::Up | MotionKind::GotoLine | MotionKind::FileEnd
        )
    }

    fn inclusive(&self) -> bool {
        match self {
            MotionKind::WordEnd { .. } | MotionKind::LineEnd => true,
            MotionKind::Find { forward, till, .. } => *forward || !*till,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum InsertKind {
    Before,
    After,
    LineStart,
    LineEnd,
    OpenBelow,
    OpenAbove,
}

#[derive(Clone, Copy, Debug)]
enum Target {
    Motion(MotionKind, usize),
    Lines(usize),
    Obj(TextObj),
    Selection,
}

#[derive(Clone, Debug)]
enum Cmd {
    Move(MotionKind, usize),
    Op { op: char, target: Target, reg: Option<char> },
    Insert(InsertKind),
    EnterVisual { linewise: bool },
    DelChar { before: bool, count: usize, reg: Option<char> },
    ToEol { op: char, reg: Option<char> },
    SubstChar { count: usize, reg: Option<char> },
    SubstLine { reg: Option<char> },
    Replace { ch: char, count: usize },
    Put { after: bool, count: usize, reg: Option<char> },
    Undo(usize),
    Redo(usize),
    Join(usize),
    StartSearch { forward: bool },
    SearchWord,
    Dot,
    Escape,
}

enum Parse {
    Incomplete,
    Invalid,
    Unsupported(String),
    Done(Cmd, usize),
}

#[derive(Clone, Debug, Default)]
struct Register {
    lines: Vec<Vec<char>>,
    linewise: bool,
}

#[derive(Clone)]
struct Snapshot {
    lines: Vec<Vec<char>>,
    cursor: Pos,
}

pub struct Engine {
    pub buf: Buffer,
    pub mode: Mode,
    pending: Vec<Key>,
    registers: HashMap<char, Register>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    visual_anchor: Pos,
    last_find: Option<(char, bool, bool)>,
    pub last_search: Option<(String, bool)>,
    /// Активная строка поиска: (введённое, вперёд).
    pub cmdline: Option<(String, bool)>,
    /// Последнее изменение в нотации клавиш — для команды `.`.
    last_change: Vec<Key>,
    recording: Option<Vec<Key>>,
    replaying: bool,
    /// Диапазон последнего изменения буфера — для подсветки «что изменилось».
    pub last_touched: Option<(usize, usize)>,
}

impl Engine {
    pub fn new(buf: Buffer) -> Self {
        Self {
            buf,
            mode: Mode::Normal,
            pending: Vec::new(),
            registers: HashMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            visual_anchor: (0, 0),
            last_find: None,
            last_search: None,
            cmdline: None,
            last_change: Vec::new(),
            recording: None,
            replaying: false,
            last_touched: None,
        }
    }

    pub fn pending_keys(&self) -> &[Key] {
        &self.pending
    }

    pub fn visual_range(&self) -> Option<(Pos, Pos)> {
        match self.mode {
            Mode::Visual | Mode::VisualLine => {
                let (a, b) = (self.visual_anchor, self.buf.cursor);
                Some(if a <= b { (a, b) } else { (b, a) })
            }
            _ => None,
        }
    }

    /// Главная точка входа: одно нажатие клавиши.
    pub fn feed(&mut self, key: Key) -> StepResult {
        if let Some(rec) = self.recording.as_mut() {
            rec.push(key);
        }
        let result = self.feed_inner(key);
        if let StepResult::Unsupported(_) = result {
            self.pending.clear();
        }
        result
    }

    fn feed_inner(&mut self, key: Key) -> StepResult {
        if self.cmdline.is_some() {
            return self.feed_cmdline(key);
        }
        match self.mode {
            Mode::Insert => self.feed_insert(key),
            _ => self.feed_normal(key),
        }
    }

    fn feed_cmdline(&mut self, key: Key) -> StepResult {
        let (mut text, forward) = self.cmdline.clone().unwrap();
        match key {
            Key::Esc => {
                self.cmdline = None;
                StepResult::Consumed
            }
            Key::Enter => {
                self.cmdline = None;
                if !text.is_empty() {
                    self.last_search = Some((text.clone(), forward));
                }
                // Идём в сторону самого поиска: `?` ищет назад, а не наоборот.
                self.do_search(true, 1);
                StepResult::Consumed
            }
            Key::Backspace => {
                text.pop();
                self.cmdline = Some((text, forward));
                StepResult::Pending
            }
            Key::Char(c) => {
                text.push(c);
                self.cmdline = Some((text, forward));
                StepResult::Pending
            }
            _ => StepResult::Ignored,
        }
    }

    fn feed_insert(&mut self, key: Key) -> StepResult {
        match key {
            Key::Esc => {
                self.mode = Mode::Normal;
                if self.buf.cursor.1 > 0 {
                    self.buf.cursor.1 -= 1;
                }
                self.buf.clamp_cursor(false);
                self.finish_change();
                StepResult::Consumed
            }
            Key::Char(c) => {
                let (row, col) = self.buf.cursor;
                let line = self.buf.line_mut(row);
                let col = col.min(line.len());
                line.insert(col, c);
                self.buf.cursor = (row, col + 1);
                self.last_touched = Some((row, row));
                StepResult::Consumed
            }
            Key::Enter => {
                let (row, col) = self.buf.cursor;
                let line = self.buf.line(row).to_vec();
                let col = col.min(line.len());
                let (head, tail) = line.split_at(col);
                let head = head.to_vec();
                let tail = tail.to_vec();
                self.buf.set_line(row, head);
                self.buf.insert_line(row + 1, tail);
                self.buf.cursor = (row + 1, 0);
                self.last_touched = Some((row, row + 1));
                StepResult::Consumed
            }
            Key::Backspace => {
                let (row, col) = self.buf.cursor;
                if col > 0 {
                    self.buf.line_mut(row).remove(col - 1);
                    self.buf.cursor = (row, col - 1);
                } else if row > 0 {
                    let cur = self.buf.remove_line(row);
                    let prev_len = self.buf.line_len(row - 1);
                    self.buf.line_mut(row - 1).extend(cur);
                    self.buf.cursor = (row - 1, prev_len);
                }
                self.last_touched = Some((row.saturating_sub(1), row));
                StepResult::Consumed
            }
            Key::Tab => {
                for _ in 0..4 {
                    self.feed_insert(Key::Char(' '));
                }
                StepResult::Consumed
            }
            Key::Ctrl(_) => StepResult::Unsupported("insert_ctrl".into()),
        }
    }

    fn feed_normal(&mut self, key: Key) -> StepResult {
        self.pending.push(key);
        let keys = self.pending.clone();
        match self.parse(&keys) {
            Parse::Incomplete => StepResult::Pending,
            Parse::Invalid => {
                self.pending.clear();
                StepResult::Ignored
            }
            Parse::Unsupported(what) => {
                self.pending.clear();
                StepResult::Unsupported(what)
            }
            Parse::Done(cmd, _used) => {
                self.pending.clear();
                self.exec(cmd, &keys);
                StepResult::Consumed
            }
        }
    }

    // ---------- разбор ----------

    fn parse(&self, keys: &[Key]) -> Parse {
        let mut i = 0;
        let mut reg = None;
        if let Some(Key::Char('"')) = keys.get(i) {
            match keys.get(i + 1) {
                None => return Parse::Incomplete,
                Some(Key::Char(c)) if c.is_ascii_alphabetic() => {
                    reg = Some(*c);
                    i += 2;
                }
                _ => return Parse::Invalid,
            }
        }
        let (count1, ni) = parse_count(keys, i);
        i = ni;
        let key = match keys.get(i) {
            None => return Parse::Incomplete,
            Some(k) => *k,
        };

        if self.mode == Mode::Visual || self.mode == Mode::VisualLine {
            if let Some(cmd) = self.parse_visual_command(key, reg) {
                return Parse::Done(cmd, i + 1);
            }
        }

        let c = match key {
            Key::Char(c) => c,
            Key::Esc => return Parse::Done(Cmd::Escape, i + 1),
            Key::Enter => {
                return Parse::Done(Cmd::Move(MotionKind::Down, count1.unwrap_or(1)), i + 1);
            }
            Key::Backspace => {
                return Parse::Done(Cmd::Move(MotionKind::Left, count1.unwrap_or(1)), i + 1);
            }
            Key::Tab => return Parse::Invalid,
            Key::Ctrl(c) => {
                return match c {
                    'r' => Parse::Done(Cmd::Redo(count1.unwrap_or(1)), i + 1),
                    'v' => Parse::Unsupported("visual_block".into()),
                    'o' | 'i' => Parse::Unsupported("jumplist".into()),
                    'd' | 'u' | 'f' | 'b' | 'e' | 'y' => Parse::Unsupported("scroll".into()),
                    _ => Parse::Invalid,
                };
            }
        };

        // операторы
        if matches!(c, 'd' | 'c' | 'y') && self.mode == Mode::Normal {
            let op = c;
            let mut j = i + 1;
            let (count2, nj) = parse_count(keys, j);
            j = nj;
            let total = count1.unwrap_or(1) * count2.unwrap_or(1);
            match keys.get(j) {
                None => return Parse::Incomplete,
                Some(Key::Char(x)) if *x == op => {
                    return Parse::Done(Cmd::Op { op, target: Target::Lines(total), reg }, j + 1);
                }
                _ => {}
            }
            return match self.parse_target(keys, j, total) {
                TargetParse::Incomplete => Parse::Incomplete,
                TargetParse::Invalid => Parse::Invalid,
                TargetParse::Unsupported(w) => Parse::Unsupported(w),
                TargetParse::Done(target, used) => Parse::Done(Cmd::Op { op, target, reg }, used),
            };
        }

        let n = count1.unwrap_or(1);
        let cmd = match c {
            'h' => Cmd::Move(MotionKind::Left, n),
            'l' | ' ' => Cmd::Move(MotionKind::Right, n),
            'j' => Cmd::Move(MotionKind::Down, n),
            'k' => Cmd::Move(MotionKind::Up, n),
            'w' => Cmd::Move(MotionKind::WordFwd { big: false }, n),
            'W' => Cmd::Move(MotionKind::WordFwd { big: true }, n),
            'b' => Cmd::Move(MotionKind::WordBack { big: false }, n),
            'B' => Cmd::Move(MotionKind::WordBack { big: true }, n),
            'e' => Cmd::Move(MotionKind::WordEnd { big: false }, n),
            'E' => Cmd::Move(MotionKind::WordEnd { big: true }, n),
            '0' => Cmd::Move(MotionKind::LineStart, 1),
            '^' => Cmd::Move(MotionKind::FirstNonBlank, 1),
            '$' => Cmd::Move(MotionKind::LineEnd, n),
            '{' => Cmd::Move(MotionKind::ParaBack, n),
            '}' => Cmd::Move(MotionKind::ParaFwd, n),
            'G' => {
                return Parse::Done(
                    Cmd::Move(
                        if count1.is_some() { MotionKind::GotoLine } else { MotionKind::FileEnd },
                        count1.unwrap_or(1),
                    ),
                    i + 1,
                );
            }
            'g' => match keys.get(i + 1) {
                None => return Parse::Incomplete,
                Some(Key::Char('g')) => {
                    return Parse::Done(Cmd::Move(MotionKind::GotoLine, count1.unwrap_or(1)), i + 2);
                }
                Some(Key::Char('e')) => return Parse::Unsupported("ge".into()),
                _ => return Parse::Invalid,
            },
            'f' | 'F' | 't' | 'T' => match keys.get(i + 1) {
                None => return Parse::Incomplete,
                Some(Key::Char(target)) => {
                    let forward = c == 'f' || c == 't';
                    let till = c == 't' || c == 'T';
                    return Parse::Done(
                        Cmd::Move(MotionKind::Find { ch: *target, forward, till }, n),
                        i + 2,
                    );
                }
                _ => return Parse::Invalid,
            },
            ';' => Cmd::Move(MotionKind::RepeatFind { reverse: false }, n),
            ',' => Cmd::Move(MotionKind::RepeatFind { reverse: true }, n),
            'n' => Cmd::Move(MotionKind::SearchNext { reverse: false }, n),
            'N' => Cmd::Move(MotionKind::SearchNext { reverse: true }, n),
            '/' => Cmd::StartSearch { forward: true },
            '?' => Cmd::StartSearch { forward: false },
            '*' => Cmd::SearchWord,
            'i' => Cmd::Insert(InsertKind::Before),
            'a' => Cmd::Insert(InsertKind::After),
            'I' => Cmd::Insert(InsertKind::LineStart),
            'A' => Cmd::Insert(InsertKind::LineEnd),
            'o' => Cmd::Insert(InsertKind::OpenBelow),
            'O' => Cmd::Insert(InsertKind::OpenAbove),
            'v' => Cmd::EnterVisual { linewise: false },
            'V' => Cmd::EnterVisual { linewise: true },
            'x' => Cmd::DelChar { before: false, count: n, reg },
            'X' => Cmd::DelChar { before: true, count: n, reg },
            'D' => Cmd::ToEol { op: 'd', reg },
            'C' => Cmd::ToEol { op: 'c', reg },
            'Y' => Cmd::Op { op: 'y', target: Target::Lines(n), reg },
            's' => Cmd::SubstChar { count: n, reg },
            'S' => Cmd::SubstLine { reg },
            'p' => Cmd::Put { after: true, count: n, reg },
            'P' => Cmd::Put { after: false, count: n, reg },
            'u' => Cmd::Undo(n),
            'J' => Cmd::Join(n),
            '.' => Cmd::Dot,
            'r' => match keys.get(i + 1) {
                None => return Parse::Incomplete,
                Some(Key::Char(ch)) => return Parse::Done(Cmd::Replace { ch: *ch, count: n }, i + 2),
                _ => return Parse::Invalid,
            },
            'm' | '\'' | '`' => return Parse::Unsupported("marks".into()),
            'q' | '@' => return Parse::Unsupported("macros".into()),
            ':' => return Parse::Unsupported("cmdline".into()),
            'Z' => return Parse::Unsupported("quit".into()),
            '>' | '<' | '=' => return Parse::Unsupported("indent".into()),
            '%' => return Parse::Unsupported("matchpair".into()),
            'R' => return Parse::Unsupported("replace_mode".into()),
            _ => return Parse::Invalid,
        };
        Parse::Done(cmd, i + 1)
    }

    fn parse_visual_command(&self, key: Key, reg: Option<char>) -> Option<Cmd> {
        let c = match key {
            Key::Char(c) => c,
            Key::Esc => return Some(Cmd::Escape),
            _ => return None,
        };
        match c {
            'd' | 'x' => Some(Cmd::Op { op: 'd', target: Target::Selection, reg }),
            'c' | 's' => Some(Cmd::Op { op: 'c', target: Target::Selection, reg }),
            'y' => Some(Cmd::Op { op: 'y', target: Target::Selection, reg }),
            'v' => Some(Cmd::EnterVisual { linewise: false }),
            'V' => Some(Cmd::EnterVisual { linewise: true }),
            _ => None,
        }
    }

    /// Цель оператора: движение или текстовый объект.
    fn parse_target(&self, keys: &[Key], i: usize, count: usize) -> TargetParse {
        let key = match keys.get(i) {
            None => return TargetParse::Incomplete,
            Some(k) => *k,
        };
        let c = match key {
            Key::Char(c) => c,
            Key::Esc => return TargetParse::Invalid,
            _ => return TargetParse::Invalid,
        };
        if c == 'i' || c == 'a' {
            let inner = c == 'i';
            let obj = match keys.get(i + 1) {
                None => return TargetParse::Incomplete,
                Some(Key::Char(o)) => match o {
                    'w' => TextObj::Word { inner, big: false },
                    'W' => TextObj::Word { inner, big: true },
                    '"' => TextObj::Quote { ch: '"', inner },
                    '\'' => TextObj::Quote { ch: '\'', inner },
                    '`' => TextObj::Quote { ch: '`', inner },
                    '(' | ')' | 'b' => TextObj::Bracket { open: '(', close: ')', inner },
                    '[' | ']' => TextObj::Bracket { open: '[', close: ']', inner },
                    '{' | '}' | 'B' => TextObj::Bracket { open: '{', close: '}', inner },
                    '<' | '>' => TextObj::Bracket { open: '<', close: '>', inner },
                    'p' => TextObj::Para { inner },
                    't' => return TargetParse::Unsupported("tag_object".into()),
                    _ => return TargetParse::Invalid,
                },
                _ => return TargetParse::Invalid,
            };
            return TargetParse::Done(Target::Obj(obj), i + 2);
        }
        let motion = match c {
            'h' => MotionKind::Left,
            'l' | ' ' => MotionKind::Right,
            'j' => MotionKind::Down,
            'k' => MotionKind::Up,
            'w' => MotionKind::WordFwd { big: false },
            'W' => MotionKind::WordFwd { big: true },
            'b' => MotionKind::WordBack { big: false },
            'B' => MotionKind::WordBack { big: true },
            'e' => MotionKind::WordEnd { big: false },
            'E' => MotionKind::WordEnd { big: true },
            '0' => MotionKind::LineStart,
            '^' => MotionKind::FirstNonBlank,
            '$' => MotionKind::LineEnd,
            '{' => MotionKind::ParaBack,
            '}' => MotionKind::ParaFwd,
            'G' => MotionKind::FileEnd,
            'n' => MotionKind::SearchNext { reverse: false },
            'N' => MotionKind::SearchNext { reverse: true },
            ';' => MotionKind::RepeatFind { reverse: false },
            ',' => MotionKind::RepeatFind { reverse: true },
            'g' => match keys.get(i + 1) {
                None => return TargetParse::Incomplete,
                Some(Key::Char('g')) => {
                    return TargetParse::Done(Target::Motion(MotionKind::GotoLine, count), i + 2);
                }
                _ => return TargetParse::Invalid,
            },
            'f' | 'F' | 't' | 'T' => match keys.get(i + 1) {
                None => return TargetParse::Incomplete,
                Some(Key::Char(target)) => {
                    let forward = c == 'f' || c == 't';
                    let till = c == 't' || c == 'T';
                    return TargetParse::Done(
                        Target::Motion(MotionKind::Find { ch: *target, forward, till }, count),
                        i + 2,
                    );
                }
                _ => return TargetParse::Invalid,
            },
            '%' => return TargetParse::Unsupported("matchpair".into()),
            _ => return TargetParse::Invalid,
        };
        TargetParse::Done(Target::Motion(motion, count), i + 1)
    }
}

enum TargetParse {
    Incomplete,
    Invalid,
    Unsupported(String),
    Done(Target, usize),
}

// ---------- исполнение ----------

impl Engine {
    fn snapshot(&self) -> Snapshot {
        Snapshot { lines: self.buf.lines_raw().clone(), cursor: self.buf.cursor }
    }

    fn push_undo(&mut self) {
        let snap = self.snapshot();
        self.undo.push(snap);
        self.redo.clear();
    }

    fn start_change_record(&mut self, keys: &[Key]) {
        if !self.replaying {
            self.recording = Some(keys.to_vec());
        }
    }

    fn set_change(&mut self, keys: &[Key]) {
        if !self.replaying {
            self.last_change = keys.to_vec();
            self.recording = None;
        }
    }

    fn finish_change(&mut self) {
        if let Some(rec) = self.recording.take() {
            self.last_change = rec;
        }
    }

    fn exec(&mut self, cmd: Cmd, keys: &[Key]) {
        self.last_touched = None;
        match cmd {
            Cmd::Move(m, n) => {
                if let Some(p) = self.apply_motion(m, n, self.buf.cursor) {
                    self.buf.cursor = p;
                }
                self.buf.clamp_cursor(false);
            }
            Cmd::Op { op, target, reg } => {
                self.push_undo();
                let enters_insert = op == 'c';
                if enters_insert {
                    self.start_change_record(keys);
                } else if op != 'y' {
                    self.set_change(keys);
                }
                if !self.apply_operator(op, target, reg) {
                    self.undo.pop();
                    self.recording = None;
                }
            }
            Cmd::Insert(kind) => {
                self.push_undo();
                self.start_change_record(keys);
                self.enter_insert(kind);
            }
            Cmd::EnterVisual { linewise } => {
                if self.mode == Mode::Normal {
                    self.visual_anchor = self.buf.cursor;
                }
                self.mode = if linewise { Mode::VisualLine } else { Mode::Visual };
            }
            Cmd::DelChar { before, count, reg } => {
                self.push_undo();
                self.set_change(keys);
                let (row, col) = self.buf.cursor;
                let len = self.buf.line_len(row);
                let (from, to) = if before {
                    (col.saturating_sub(count), col)
                } else {
                    (col, (col + count).min(len))
                };
                if from >= to {
                    self.undo.pop();
                    return;
                }
                let removed: Vec<char> = self.buf.line_mut(row).drain(from..to).collect();
                self.store_register(reg, vec![removed], false);
                self.buf.cursor = (row, from);
                self.buf.clamp_cursor(false);
                self.last_touched = Some((row, row));
            }
            Cmd::ToEol { op, reg } => {
                self.push_undo();
                if op == 'c' {
                    self.start_change_record(keys);
                } else {
                    self.set_change(keys);
                }
                let (row, col) = self.buf.cursor;
                let len = self.buf.line_len(row);
                let removed: Vec<char> = if col < len {
                    self.buf.line_mut(row).drain(col..len).collect()
                } else {
                    Vec::new()
                };
                self.store_register(reg, vec![removed], false);
                self.last_touched = Some((row, row));
                if op == 'c' {
                    self.mode = Mode::Insert;
                    self.buf.cursor = (row, col);
                } else {
                    self.buf.clamp_cursor(false);
                }
            }
            Cmd::SubstChar { count, reg } => {
                self.push_undo();
                self.start_change_record(keys);
                let (row, col) = self.buf.cursor;
                let len = self.buf.line_len(row);
                let to = (col + count).min(len);
                let removed: Vec<char> = self.buf.line_mut(row).drain(col..to).collect();
                self.store_register(reg, vec![removed], false);
                self.mode = Mode::Insert;
                self.last_touched = Some((row, row));
            }
            Cmd::SubstLine { reg } => {
                self.push_undo();
                self.start_change_record(keys);
                let row = self.buf.cursor.0;
                let old = self.buf.line(row).to_vec();
                let indent: Vec<char> =
                    old.iter().take_while(|c| c.is_whitespace()).copied().collect();
                self.store_register(reg, vec![old], true);
                let indent_len = indent.len();
                self.buf.set_line(row, indent);
                self.buf.cursor = (row, indent_len);
                self.mode = Mode::Insert;
                self.last_touched = Some((row, row));
            }
            Cmd::Replace { ch, count } => {
                let (row, col) = self.buf.cursor;
                if col + count > self.buf.line_len(row) {
                    return;
                }
                self.push_undo();
                self.set_change(keys);
                for i in 0..count {
                    self.buf.line_mut(row)[col + i] = ch;
                }
                self.buf.cursor = (row, col + count - 1);
                self.last_touched = Some((row, row));
            }
            Cmd::Put { after, count, reg } => {
                let register = match self.register(reg) {
                    Some(r) if !r.lines.is_empty() => r,
                    _ => return,
                };
                self.push_undo();
                self.set_change(keys);
                self.put(register, after, count);
            }
            Cmd::Undo(n) => {
                for _ in 0..n {
                    if let Some(snap) = self.undo.pop() {
                        let cur = self.snapshot();
                        self.redo.push(cur);
                        self.buf.set_lines(snap.lines);
                        self.buf.cursor = snap.cursor;
                        self.buf.clamp_cursor(false);
                    }
                }
            }
            Cmd::Redo(n) => {
                for _ in 0..n {
                    if let Some(snap) = self.redo.pop() {
                        let cur = self.snapshot();
                        self.undo.push(cur);
                        self.buf.set_lines(snap.lines);
                        self.buf.cursor = snap.cursor;
                        self.buf.clamp_cursor(false);
                    }
                }
            }
            Cmd::Join(n) => {
                let joins = n.max(2) - 1;
                self.push_undo();
                self.set_change(keys);
                let row = self.buf.cursor.0;
                for _ in 0..joins {
                    if row + 1 >= self.buf.line_count() {
                        break;
                    }
                    let next = self.buf.remove_line(row + 1);
                    let trimmed: Vec<char> =
                        next.iter().skip_while(|c| c.is_whitespace()).copied().collect();
                    let len = self.buf.line_len(row);
                    let line = self.buf.line_mut(row);
                    if !line.is_empty() && !trimmed.is_empty() {
                        line.push(' ');
                    }
                    line.extend(trimmed);
                    self.buf.cursor = (row, len);
                }
                self.last_touched = Some((row, row));
            }
            Cmd::StartSearch { forward } => {
                self.cmdline = Some((String::new(), forward));
            }
            Cmd::SearchWord => {
                if let Some(word) = motion::word_under_cursor(&self.buf, self.buf.cursor) {
                    self.last_search = Some((word, true));
                    self.do_search(true, 1);
                }
            }
            Cmd::Dot => {
                let change = self.last_change.clone();
                if change.is_empty() {
                    return;
                }
                self.replaying = true;
                for k in change {
                    self.feed_inner(k);
                }
                self.replaying = false;
            }
            Cmd::Escape => {
                self.mode = Mode::Normal;
                self.pending.clear();
                self.buf.clamp_cursor(false);
            }
        }
    }

    fn enter_insert(&mut self, kind: InsertKind) {
        let (row, col) = self.buf.cursor;
        match kind {
            InsertKind::Before => {}
            InsertKind::After => self.buf.cursor = (row, (col + 1).min(self.buf.line_len(row))),
            InsertKind::LineStart => self.buf.cursor = (row, self.buf.first_non_blank(row)),
            InsertKind::LineEnd => self.buf.cursor = (row, self.buf.line_len(row)),
            InsertKind::OpenBelow => {
                let indent = self.indent_of(row);
                let len = indent.len();
                self.buf.insert_line(row + 1, indent);
                self.buf.cursor = (row + 1, len);
                self.last_touched = Some((row + 1, row + 1));
            }
            InsertKind::OpenAbove => {
                let indent = self.indent_of(row);
                let len = indent.len();
                self.buf.insert_line(row, indent);
                self.buf.cursor = (row, len);
                self.last_touched = Some((row, row));
            }
        }
        self.mode = Mode::Insert;
    }

    fn indent_of(&self, row: usize) -> Vec<char> {
        self.buf.line(row).iter().take_while(|c| c.is_whitespace()).copied().collect()
    }

    fn apply_motion(&mut self, m: MotionKind, count: usize, from: Pos) -> Option<Pos> {
        let mut pos = from;
        for _ in 0..count.max(1) {
            pos = self.apply_motion_once(m, pos, count)?;
        }
        Some(pos)
    }

    fn apply_motion_once(&mut self, m: MotionKind, pos: Pos, count: usize) -> Option<Pos> {
        let (row, col) = pos;
        let p = match m {
            MotionKind::Left => (row, col.saturating_sub(1)),
            MotionKind::Right => (row, (col + 1).min(self.buf.max_col(row, false))),
            MotionKind::Down => {
                if row + 1 >= self.buf.line_count() {
                    return None;
                }
                (row + 1, col.min(self.buf.max_col(row + 1, false)))
            }
            MotionKind::Up => {
                if row == 0 {
                    return None;
                }
                (row - 1, col.min(self.buf.max_col(row - 1, false)))
            }
            MotionKind::WordFwd { big } => motion::next_word_start(&self.buf, pos, big),
            MotionKind::WordBack { big } => motion::prev_word_start(&self.buf, pos, big),
            MotionKind::WordEnd { big } => motion::word_end(&self.buf, pos, big),
            MotionKind::LineStart => (row, 0),
            MotionKind::FirstNonBlank => (row, self.buf.first_non_blank(row)),
            MotionKind::LineEnd => (row, self.buf.max_col(row, false)),
            MotionKind::GotoLine => {
                let target = count.clamp(1, self.buf.line_count()) - 1;
                return Some((target, self.buf.first_non_blank(target)));
            }
            MotionKind::FileEnd => {
                let last = self.buf.line_count() - 1;
                return Some((last, self.buf.first_non_blank(last)));
            }
            MotionKind::ParaFwd => motion::para_fwd(&self.buf, pos),
            MotionKind::ParaBack => motion::para_back(&self.buf, pos),
            MotionKind::Find { ch, forward, till } => {
                self.last_find = Some((ch, forward, till));
                motion::find_in_line(&self.buf, pos, ch, forward, till)?
            }
            MotionKind::RepeatFind { reverse } => {
                let (ch, forward, till) = self.last_find?;
                let forward = if reverse { !forward } else { forward };
                motion::find_in_line(&self.buf, pos, ch, forward, till)?
            }
            MotionKind::SearchNext { reverse } => {
                let (pattern, forward) = self.last_search.clone()?;
                let forward = if reverse { !forward } else { forward };
                motion::search(&self.buf, pos, &pattern, forward)?
            }
        };
        Some(p)
    }

    fn do_search(&mut self, forward: bool, count: usize) {
        let Some((pattern, dir)) = self.last_search.clone() else { return };
        let forward = if forward { dir } else { !dir };
        let mut pos = self.buf.cursor;
        for _ in 0..count {
            match motion::search(&self.buf, pos, &pattern, forward) {
                Some(p) => pos = p,
                None => return,
            }
        }
        self.buf.cursor = pos;
    }

    /// Возвращает включительный диапазон, на который действует оператор.
    fn resolve_target(&mut self, op: char, target: Target) -> Option<(Pos, Pos, bool)> {
        match target {
            Target::Lines(n) => {
                let start = self.buf.cursor.0;
                let end = (start + n - 1).min(self.buf.line_count() - 1);
                Some(((start, 0), (end, self.buf.line_len(end).saturating_sub(1)), true))
            }
            Target::Obj(obj) => motion::textobj_range(&self.buf, self.buf.cursor, obj),
            Target::Selection => {
                let (a, b) = self.visual_range()?;
                let linewise = self.mode == Mode::VisualLine;
                if linewise {
                    Some(((a.0, 0), (b.0, self.buf.line_len(b.0).saturating_sub(1)), true))
                } else {
                    Some((a, b, false))
                }
            }
            Target::Motion(m, count) => {
                // Знаменитое исключение: `cw` на непробельном символе ведёт себя как `ce`.
                let m = match (op, m) {
                    ('c', MotionKind::WordFwd { big })
                        if self.buf.cur_char().map(|c| !c.is_whitespace()).unwrap_or(false) =>
                    {
                        MotionKind::WordEnd { big }
                    }
                    _ => m,
                };
                let start = self.buf.cursor;
                let end = self.apply_motion(m, count, start)?;
                if m.linewise() {
                    let (a, b) = if start.0 <= end.0 { (start.0, end.0) } else { (end.0, start.0) };
                    return Some(((a, 0), (b, self.buf.line_len(b).saturating_sub(1)), true));
                }
                let (mut s, mut e) = if start <= end { (start, end) } else { (end, start) };
                if !m.inclusive() {
                    if s == e {
                        return None;
                    }
                    // исключающее движение: последний символ не входит
                    if e.1 == 0 {
                        if e.0 == 0 {
                            return None;
                        }
                        e = (e.0 - 1, self.buf.line_len(e.0 - 1).saturating_sub(1));
                        if e < s {
                            return None;
                        }
                    } else {
                        e = (e.0, e.1 - 1);
                    }
                }
                if s > e {
                    std::mem::swap(&mut s, &mut e);
                }
                Some((s, e, false))
            }
        }
    }

    fn apply_operator(&mut self, op: char, target: Target, reg: Option<char>) -> bool {
        let Some((start, end, linewise)) = self.resolve_target(op, target) else {
            return false;
        };
        let text = self.extract(start, end, linewise);
        self.store_register(reg, text, linewise);
        match op {
            'y' => {
                if self.mode == Mode::Visual || self.mode == Mode::VisualLine {
                    self.mode = Mode::Normal;
                }
                self.buf.cursor = if linewise { (start.0, self.buf.cursor.1) } else { start };
                self.buf.clamp_cursor(false);
            }
            'd' | 'c' => {
                let change = op == 'c';
                self.delete_range(start, end, linewise, change);
                if self.mode == Mode::Visual || self.mode == Mode::VisualLine {
                    self.mode = Mode::Normal;
                }
                if change {
                    self.mode = Mode::Insert;
                } else {
                    self.buf.clamp_cursor(false);
                }
            }
            _ => return false,
        }
        self.last_touched = Some((start.0, end.0));
        true
    }

    fn extract(&self, start: Pos, end: Pos, linewise: bool) -> Vec<Vec<char>> {
        if linewise {
            (start.0..=end.0).map(|r| self.buf.line(r).to_vec()).collect()
        } else if start.0 == end.0 {
            let line = self.buf.line(start.0);
            let e = (end.1 + 1).min(line.len());
            vec![line[start.1.min(line.len())..e].to_vec()]
        } else {
            let mut out = Vec::new();
            out.push(self.buf.line(start.0)[start.1..].to_vec());
            for r in start.0 + 1..end.0 {
                out.push(self.buf.line(r).to_vec());
            }
            let last = self.buf.line(end.0);
            out.push(last[..(end.1 + 1).min(last.len())].to_vec());
            out
        }
    }

    fn delete_range(&mut self, start: Pos, end: Pos, linewise: bool, change: bool) {
        if linewise {
            if change {
                let indent = self.indent_of(start.0);
                for r in (start.0..=end.0).rev() {
                    self.buf.remove_line(r);
                }
                let len = indent.len();
                self.buf.insert_line(start.0, indent);
                self.buf.cursor = (start.0, len);
            } else {
                for r in (start.0..=end.0).rev() {
                    self.buf.remove_line(r);
                }
                let row = start.0.min(self.buf.line_count() - 1);
                self.buf.cursor = (row, self.buf.first_non_blank(row));
            }
            return;
        }
        if start.0 == end.0 {
            let len = self.buf.line_len(start.0);
            let e = (end.1 + 1).min(len);
            let s = start.1.min(len);
            self.buf.line_mut(start.0).drain(s..e);
        } else {
            let head: Vec<char> = self.buf.line(start.0)[..start.1].to_vec();
            let last = self.buf.line(end.0).to_vec();
            let tail: Vec<char> = last[(end.1 + 1).min(last.len())..].to_vec();
            for r in (start.0 + 1..=end.0).rev() {
                self.buf.remove_line(r);
            }
            let mut merged = head;
            merged.extend(tail);
            self.buf.set_line(start.0, merged);
        }
        self.buf.cursor = start;
    }

    fn store_register(&mut self, reg: Option<char>, lines: Vec<Vec<char>>, linewise: bool) {
        let value = Register { lines, linewise };
        if let Some(r) = reg {
            self.registers.insert(r, value.clone());
        }
        self.registers.insert('"', value);
    }

    fn register(&self, reg: Option<char>) -> Option<Register> {
        self.registers.get(&reg.unwrap_or('"')).cloned()
    }

    fn put(&mut self, register: Register, after: bool, count: usize) {
        let (row, col) = self.buf.cursor;
        if register.linewise {
            let at = if after { row + 1 } else { row };
            let mut inserted = 0;
            for _ in 0..count {
                for line in register.lines.iter() {
                    self.buf.insert_line(at + inserted, line.clone());
                    inserted += 1;
                }
            }
            self.buf.cursor = (at, self.buf.first_non_blank(at));
            self.last_touched = Some((at, at + inserted.saturating_sub(1)));
            return;
        }
        let at = if after { (col + 1).min(self.buf.line_len(row)) } else { col };
        if register.lines.len() == 1 {
            let mut chunk: Vec<char> = Vec::new();
            for _ in 0..count {
                chunk.extend(register.lines[0].iter().copied());
            }
            let n = chunk.len();
            let line = self.buf.line_mut(row);
            for (i, c) in chunk.into_iter().enumerate() {
                line.insert(at + i, c);
            }
            self.buf.cursor = (row, at + n.saturating_sub(1));
            self.last_touched = Some((row, row));
        } else {
            let line = self.buf.line(row).to_vec();
            let (head, tail) = line.split_at(at.min(line.len()));
            let mut first = head.to_vec();
            first.extend(register.lines[0].iter().copied());
            self.buf.set_line(row, first);
            let mut at_row = row;
            for line in register.lines[1..].iter() {
                at_row += 1;
                self.buf.insert_line(at_row, line.clone());
            }
            let last_len = self.buf.line_len(at_row);
            self.buf.line_mut(at_row).extend(tail.iter().copied());
            self.buf.cursor = (at_row, last_len.saturating_sub(1));
            self.last_touched = Some((row, at_row));
        }
    }
}

/// Считывает счётчик перед командой: `3dd`, `d2w`. Ноль в начале — это `0`
/// (движение в начало строки), а не цифра счётчика.
fn parse_count(keys: &[Key], mut i: usize) -> (Option<usize>, usize) {
    let mut n: Option<usize> = None;
    while let Some(Key::Char(c)) = keys.get(i) {
        if c.is_ascii_digit() && !(*c == '0' && n.is_none()) {
            let d = c.to_digit(10).unwrap() as usize;
            n = Some(n.unwrap_or(0) * 10 + d);
            i += 1;
        } else {
            break;
        }
    }
    (n, i)
}
