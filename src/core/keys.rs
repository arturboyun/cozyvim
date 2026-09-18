//! Нажатия клавиш в vim-нотации: разбор строк вроде `"d2w<Esc>"` и обратная
//! сборка, чтобы решение урока и ввод пользователя сравнивались как текст.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Esc,
    Enter,
    Backspace,
    Tab,
}

impl Key {
    /// Нотация клавиши: то, что автор урока пишет в `solutions`.
    pub fn notation(&self) -> String {
        match self {
            Key::Char(' ') => "<Space>".into(),
            Key::Char(c) => c.to_string(),
            Key::Ctrl(c) => format!("<C-{c}>"),
            Key::Esc => "<Esc>".into(),
            Key::Enter => "<CR>".into(),
            Key::Backspace => "<BS>".into(),
            Key::Tab => "<Tab>".into(),
        }
    }

    /// Короткая подпись для экранной клавиатуры и ленты нажатий.
    pub fn label(&self) -> String {
        match self {
            Key::Char(' ') => "␣".into(),
            Key::Char(c) => c.to_string(),
            Key::Ctrl(c) => format!("^{c}"),
            Key::Esc => "Esc".into(),
            Key::Enter => "⏎".into(),
            Key::Backspace => "⌫".into(),
            Key::Tab => "⇥".into(),
        }
    }
}

/// Разбирает vim-нотацию в последовательность клавиш.
/// Неизвестный `<...>`-токен считается литеральными символами.
pub fn parse_keys(s: &str) -> Vec<Key> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            if let Some(end) = chars[i..].iter().position(|c| *c == '>') {
                let token: String = chars[i + 1..i + end].iter().collect();
                if let Some(key) = parse_token(&token) {
                    out.push(key);
                    i += end + 1;
                    continue;
                }
            }
        }
        out.push(Key::Char(chars[i]));
        i += 1;
    }
    out
}

fn parse_token(token: &str) -> Option<Key> {
    let lower = token.to_ascii_lowercase();
    match lower.as_str() {
        "esc" => Some(Key::Esc),
        "cr" | "enter" | "return" => Some(Key::Enter),
        "bs" => Some(Key::Backspace),
        "tab" => Some(Key::Tab),
        "space" => Some(Key::Char(' ')),
        "lt" => Some(Key::Char('<')),
        "gt" => Some(Key::Char('>')),
        _ => {
            let rest = lower.strip_prefix("c-")?;
            let mut it = rest.chars();
            let c = it.next()?;
            if it.next().is_some() { None } else { Some(Key::Ctrl(c)) }
        }
    }
}

pub fn keys_notation(keys: &[Key]) -> String {
    keys.iter().map(|k| k.notation()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_tokens() {
        assert_eq!(parse_keys("dw"), vec![Key::Char('d'), Key::Char('w')]);
        assert_eq!(
            parse_keys("ciw<Esc>"),
            vec![Key::Char('c'), Key::Char('i'), Key::Char('w'), Key::Esc]
        );
        assert_eq!(parse_keys("<C-r>"), vec![Key::Ctrl('r')]);
    }

    #[test]
    fn notation_roundtrips() {
        for s in ["dw", "d2w", "ciwfoo<Esc>", "<C-r>u", "x<Space>"] {
            assert_eq!(keys_notation(&parse_keys(s)), s);
        }
    }

    #[test]
    fn unknown_token_stays_literal() {
        assert_eq!(parse_keys("<zz>").len(), 4);
    }
}
