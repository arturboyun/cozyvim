//! Текстовый буфер тренажёра: строки как `Vec<char>`, чтобы кириллица
//! и любые не-ASCII символы вели себя как один символ, а не как байты.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Buffer {
    lines: Vec<Vec<char>>,
    /// (строка, столбец) — обе координаты от нуля.
    pub cursor: (usize, usize),
}

impl Buffer {
    pub fn new(lines: &[String]) -> Self {
        let mut lines: Vec<Vec<char>> = lines.iter().map(|l| l.chars().collect()).collect();
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        Self { lines, cursor: (0, 0) }
    }

    pub fn from_str_slice(lines: &[&str]) -> Self {
        Self::new(&lines.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    pub fn to_lines(&self) -> Vec<String> {
        self.lines.iter().map(|l| l.iter().collect()).collect()
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, row: usize) -> &[char] {
        self.lines.get(row).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn line_mut(&mut self, row: usize) -> &mut Vec<char> {
        &mut self.lines[row]
    }

    pub fn line_len(&self, row: usize) -> usize {
        self.lines.get(row).map(|l| l.len()).unwrap_or(0)
    }

    pub fn char_at(&self, row: usize, col: usize) -> Option<char> {
        self.lines.get(row).and_then(|l| l.get(col)).copied()
    }

    pub fn cur_char(&self) -> Option<char> {
        self.char_at(self.cursor.0, self.cursor.1)
    }

    pub fn insert_line(&mut self, at: usize, line: Vec<char>) {
        let at = at.min(self.lines.len());
        self.lines.insert(at, line);
    }

    pub fn remove_line(&mut self, at: usize) -> Vec<char> {
        let removed = self.lines.remove(at);
        if self.lines.is_empty() {
            self.lines.push(Vec::new());
        }
        removed
    }

    pub fn set_line(&mut self, at: usize, line: Vec<char>) {
        self.lines[at] = line;
    }

    pub fn lines_raw(&self) -> &Vec<Vec<char>> {
        &self.lines
    }

    pub fn set_lines(&mut self, lines: Vec<Vec<char>>) {
        self.lines = if lines.is_empty() { vec![Vec::new()] } else { lines };
    }

    /// Последний допустимый столбец. В нормальном режиме курсор стоит *на*
    /// символе, поэтому предел на единицу меньше, чем в режиме вставки.
    pub fn max_col(&self, row: usize, insert_mode: bool) -> usize {
        let len = self.line_len(row);
        if insert_mode { len } else { len.saturating_sub(1) }
    }

    pub fn clamp_cursor(&mut self, insert_mode: bool) {
        let (mut row, mut col) = self.cursor;
        row = row.min(self.lines.len().saturating_sub(1));
        col = col.min(self.max_col(row, insert_mode));
        self.cursor = (row, col);
    }

    pub fn first_non_blank(&self, row: usize) -> usize {
        let line = self.line(row);
        line.iter().position(|c| !c.is_whitespace()).unwrap_or(0)
    }
}
