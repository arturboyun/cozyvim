//! Настройки приложения. Лежат рядом с журналом прогресса, пишутся сразу
//! после изменения — приложение без «сохранить».

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    #[serde(rename = "ru")]
    Ru,
    #[serde(rename = "en")]
    En,
}

impl Lang {
    pub fn code(&self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Lang::Ru => "Русский",
            Lang::En => "English",
        }
    }
}

/// Как тренажёр засчитывает упражнение.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckMode {
    /// Зачёт по результату, звёзды — за оптимальность. По умолчанию.
    Mix,
    /// Зачёт только за точное совпадение с одним из решений.
    Strict,
    /// Зачёт по результату, оптимум показываем справочно.
    Free,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyboardVisibility {
    /// Показывать, пока навык не вырос, потом прятать самостоятельно.
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Experience {
    Newcomer,
    Dabbler,
    Confident,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub lang: Lang,
    pub check_mode: CheckMode,
    pub theme: Theme,
    pub keyboard: KeyboardVisibility,
    pub experience: Experience,
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lang: Lang::Ru,
            check_mode: CheckMode::Mix,
            theme: Theme::Dark,
            keyboard: KeyboardVisibility::Auto,
            experience: Experience::Newcomer,
            onboarded: false,
        }
    }
}

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("COZYVIM_DATA_DIR") {
        return PathBuf::from(dir);
    }
    directories::ProjectDirs::from("", "", "cozyvim")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".cozyvim"))
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
