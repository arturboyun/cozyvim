//! Журнал событий: одна строка JSON на попытку. Формат намеренно
//! append-only — графики и статистика считаются поверх сырых событий,
//! поэтому новые метрики не требуют миграций.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::settings::{CheckMode, data_dir};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    /// Момент попытки в RFC 3339.
    pub ts: String,
    pub lesson_id: String,
    pub exercise_index: usize,
    pub mode: CheckMode,
    /// Нажатия в vim-нотации — восстановить их задним числом невозможно.
    pub keystrokes: String,
    pub keystroke_count: usize,
    pub optimal: usize,
    pub success: bool,
    pub stars: u8,
    pub elapsed_ms: u64,
    /// Эталонное решение — чтобы статистика «проблемные команды» знала,
    /// о какой команде речь, даже если урок потом изменится.
    pub canonical: String,
}

fn journal_path() -> PathBuf {
    data_dir().join("events.jsonl")
}

#[derive(Debug, Clone, Default)]
pub struct Journal {
    pub attempts: Vec<Attempt>,
}

impl Journal {
    pub fn load() -> Self {
        let path = journal_path();
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let attempts = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Attempt>(l).ok())
            .collect();
        Self { attempts }
    }

    pub fn append(&mut self, attempt: Attempt) {
        let path = journal_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(line) = serde_json::to_string(&attempt) {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                let _ = writeln!(f, "{line}");
            }
        }
        self.attempts.push(attempt);
    }

    pub fn clear(&mut self) {
        self.attempts.clear();
        let _ = std::fs::remove_file(journal_path());
    }
}
