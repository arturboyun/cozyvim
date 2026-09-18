//! Вычисление позиций: движения по словам, строкам, абзацам и границы
//! текстовых объектов. Всё работает над `Buffer` и координатами (строка, столбец).

use crate::core::buffer::Buffer;

pub type Pos = (usize, usize);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Class {
    Blank,
    Word,
    Punct,
}

fn class(c: char, big: bool) -> Class {
    if c.is_whitespace() {
        Class::Blank
    } else if big {
        Class::Word
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Punct
    }
}

fn char_at(buf: &Buffer, p: Pos) -> Option<char> {
    buf.char_at(p.0, p.1)
}

/// Следующая позиция в буфере; конец строки трактуется как виртуальный перевод.
fn step_fwd(buf: &Buffer, p: Pos) -> Option<Pos> {
    let (row, col) = p;
    if col + 1 < buf.line_len(row) {
        Some((row, col + 1))
    } else if col < buf.line_len(row) {
        // позиция "на переводе строки"
        if row + 1 < buf.line_count() { Some((row + 1, 0)) } else { None }
    } else if row + 1 < buf.line_count() {
        Some((row + 1, 0))
    } else {
        None
    }
}

fn step_back(buf: &Buffer, p: Pos) -> Option<Pos> {
    let (row, col) = p;
    if col > 0 {
        Some((row, col - 1))
    } else if row > 0 {
        let prev = row - 1;
        Some((prev, buf.line_len(prev).saturating_sub(1)))
    } else {
        None
    }
}

pub fn next_word_start(buf: &Buffer, from: Pos, big: bool) -> Pos {
    let mut p = from;
    let start_class = char_at(buf, p).map(|c| class(c, big));
    // уходим с текущего слова
    if let Some(sc) = start_class {
        if sc != Class::Blank {
            while let Some(np) = step_fwd(buf, p) {
                match char_at(buf, np).map(|c| class(c, big)) {
                    Some(c) if c == sc => p = np,
                    _ => {
                        p = np;
                        break;
                    }
                }
            }
        }
    }
    // пропускаем пробелы и пустые строки
    loop {
        match char_at(buf, p) {
            Some(c) if class(c, big) == Class::Blank => {}
            Some(_) => break,
            None => {
                // пустая строка — сама по себе слово
                if buf.line_len(p.0) == 0 && p != from {
                    break;
                }
            }
        }
        match step_fwd(buf, p) {
            Some(np) => p = np,
            None => break,
        }
    }
    p
}

pub fn prev_word_start(buf: &Buffer, from: Pos, big: bool) -> Pos {
    let mut p = match step_back(buf, from) {
        Some(p) => p,
        None => return from,
    };
    // назад через пробелы
    while char_at(buf, p).map(|c| class(c, big)) == Some(Class::Blank) || char_at(buf, p).is_none() {
        if buf.line_len(p.0) == 0 {
            return p;
        }
        match step_back(buf, p) {
            Some(np) => p = np,
            None => return p,
        }
    }
    let target = char_at(buf, p).map(|c| class(c, big));
    while let Some(np) = step_back(buf, p) {
        if char_at(buf, np).map(|c| class(c, big)) == target && np.0 == p.0 {
            p = np;
        } else {
            break;
        }
    }
    p
}

pub fn word_end(buf: &Buffer, from: Pos, big: bool) -> Pos {
    let mut p = match step_fwd(buf, from) {
        Some(p) => p,
        None => return from,
    };
    while char_at(buf, p).map(|c| class(c, big)) == Some(Class::Blank) || char_at(buf, p).is_none() {
        match step_fwd(buf, p) {
            Some(np) => p = np,
            None => return p,
        }
    }
    let target = char_at(buf, p).map(|c| class(c, big));
    while let Some(np) = step_fwd(buf, p) {
        if np.0 == p.0 && char_at(buf, np).map(|c| class(c, big)) == target {
            p = np;
        } else {
            break;
        }
    }
    p
}

pub fn find_in_line(buf: &Buffer, from: Pos, ch: char, forward: bool, till: bool) -> Option<Pos> {
    let (row, col) = from;
    let line = buf.line(row);
    if forward {
        let start = col + 1;
        let extra = if till { 1 } else { 0 };
        let start = start + extra;
        let idx = line.iter().skip(start).position(|c| *c == ch)? + start;
        Some((row, if till { idx - 1 } else { idx }))
    } else {
        let idx = line[..col].iter().rposition(|c| *c == ch)?;
        Some((row, if till { idx + 1 } else { idx }))
    }
}

pub fn para_fwd(buf: &Buffer, from: Pos) -> Pos {
    let mut row = from.0 + 1;
    while row < buf.line_count() {
        if buf.line_len(row) == 0 {
            return (row, 0);
        }
        row += 1;
    }
    (buf.line_count() - 1, buf.line_len(buf.line_count() - 1).saturating_sub(1))
}

pub fn para_back(buf: &Buffer, from: Pos) -> Pos {
    let mut row = from.0;
    while row > 0 {
        row -= 1;
        if buf.line_len(row) == 0 {
            return (row, 0);
        }
    }
    (0, 0)
}

/// Поиск подстроки по буферу с заворотом.
pub fn search(buf: &Buffer, from: Pos, pattern: &str, forward: bool) -> Option<Pos> {
    if pattern.is_empty() {
        return None;
    }
    let pat: Vec<char> = pattern.chars().collect();
    let count = buf.line_count();
    let matches_at = |row: usize, col: usize| -> bool {
        let line = buf.line(row);
        col + pat.len() <= line.len() && line[col..col + pat.len()] == pat[..]
    };
    if forward {
        for i in 0..=count {
            let row = (from.0 + i) % count;
            let start = if i == 0 { from.1 + 1 } else { 0 };
            for col in start..buf.line_len(row) {
                if matches_at(row, col) {
                    return Some((row, col));
                }
            }
        }
    } else {
        for i in 0..=count {
            let row = (from.0 + count - (i % count)) % count;
            let end = if i == 0 { from.1 } else { buf.line_len(row) };
            for col in (0..end).rev() {
                if matches_at(row, col) {
                    return Some((row, col));
                }
            }
        }
    }
    None
}

/// Слово под курсором — для команды `*`.
pub fn word_under_cursor(buf: &Buffer, pos: Pos) -> Option<String> {
    let line = buf.line(pos.0);
    if line.is_empty() {
        return None;
    }
    let col = pos.1.min(line.len() - 1);
    if class(line[col], false) != Class::Word {
        return None;
    }
    let mut start = col;
    while start > 0 && class(line[start - 1], false) == Class::Word {
        start -= 1;
    }
    let mut end = col;
    while end + 1 < line.len() && class(line[end + 1], false) == Class::Word {
        end += 1;
    }
    Some(line[start..=end].iter().collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextObj {
    Word { inner: bool, big: bool },
    Quote { ch: char, inner: bool },
    Bracket { open: char, close: char, inner: bool },
    Para { inner: bool },
}

/// Диапазон текстового объекта: включительные позиции начала и конца.
pub fn textobj_range(buf: &Buffer, pos: Pos, obj: TextObj) -> Option<(Pos, Pos, bool)> {
    match obj {
        TextObj::Word { inner, big } => {
            let line = buf.line(pos.0);
            if line.is_empty() {
                return None;
            }
            let col = pos.1.min(line.len() - 1);
            let target = class(line[col], big);
            let mut start = col;
            while start > 0 && class(line[start - 1], big) == target {
                start -= 1;
            }
            let mut end = col;
            while end + 1 < line.len() && class(line[end + 1], big) == target {
                end += 1;
            }
            if !inner {
                // `aw` захватывает пробелы справа, иначе слева
                let mut e = end;
                let mut grew = false;
                while e + 1 < line.len() && class(line[e + 1], big) == Class::Blank {
                    e += 1;
                    grew = true;
                }
                if grew {
                    end = e;
                } else {
                    while start > 0 && class(line[start - 1], big) == Class::Blank {
                        start -= 1;
                    }
                }
            }
            Some(((pos.0, start), (pos.0, end), false))
        }
        TextObj::Quote { ch, inner } => {
            let line = buf.line(pos.0);
            let mut positions: Vec<usize> = Vec::new();
            for (i, c) in line.iter().enumerate() {
                if *c == ch {
                    positions.push(i);
                }
            }
            let pair = positions
                .chunks(2)
                .find(|ch| ch.len() == 2 && ch[1] >= pos.1)
                .map(|ch| (ch[0], ch[1]))?;
            if inner {
                if pair.1 == pair.0 + 1 {
                    return None;
                }
                Some(((pos.0, pair.0 + 1), (pos.0, pair.1 - 1), false))
            } else {
                Some(((pos.0, pair.0), (pos.0, pair.1), false))
            }
        }
        TextObj::Bracket { open, close, inner } => {
            let (start, end) = find_bracket_pair(buf, pos, open, close)?;
            if inner {
                let s = step_fwd(buf, start)?;
                let e = step_back(buf, end)?;
                if s > e {
                    return None;
                }
                Some((s, e, false))
            } else {
                Some((start, end, false))
            }
        }
        TextObj::Para { inner } => {
            let mut top = pos.0;
            while top > 0 && buf.line_len(top - 1) > 0 {
                top -= 1;
            }
            let mut bot = pos.0;
            while bot + 1 < buf.line_count() && buf.line_len(bot + 1) > 0 {
                bot += 1;
            }
            if !inner {
                while bot + 1 < buf.line_count() && buf.line_len(bot + 1) == 0 {
                    bot += 1;
                }
            }
            Some(((top, 0), (bot, buf.line_len(bot).saturating_sub(1)), true))
        }
    }
}

fn find_bracket_pair(buf: &Buffer, pos: Pos, open: char, close: char) -> Option<(Pos, Pos)> {
    // ищем открывающую скобку назад от курсора
    let mut depth = 0i32;
    let mut p = pos;
    let start = loop {
        let c = char_at(buf, p);
        if c == Some(close) && p != pos {
            depth += 1;
        } else if c == Some(open) {
            if depth == 0 {
                break p;
            }
            depth -= 1;
        }
        p = step_back(buf, p)?;
    };
    // и закрывающую вперёд
    let mut depth = 0i32;
    let mut p = start;
    let end = loop {
        p = step_fwd(buf, p)?;
        let c = char_at(buf, p);
        if c == Some(open) {
            depth += 1;
        } else if c == Some(close) {
            if depth == 0 {
                break p;
            }
            depth -= 1;
        }
    };
    Some((start, end))
}
