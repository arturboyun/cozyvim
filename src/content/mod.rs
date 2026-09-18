//! Загрузка уроков. Механика упражнения описана один раз, тексты — секциями
//! по языкам, поэтому перевод не может разъехаться с логикой.

use std::collections::HashMap;

use include_dir::{Dir, include_dir};
use serde::Deserialize;

use crate::core::keys::{Key, parse_keys};

static CONTENT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/content");

#[derive(Debug, Clone, Deserialize)]
pub struct LocalizedText {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub task: String,
    #[serde(default)]
    pub hint: String,
}

impl LocalizedText {
    pub fn empty() -> Self {
        Self {
            title: String::new(),
            subtitle: String::new(),
            intro: String::new(),
            task: String::new(),
            hint: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Exercise {
    pub buffer: Vec<String>,
    #[serde(default)]
    pub cursor: [usize; 2],
    pub goal: Vec<String>,
    /// Для уроков про движение результат — позиция курсора, а не текст.
    #[serde(default)]
    pub goal_cursor: Option<[usize; 2]>,
    pub solutions: Vec<String>,
    #[serde(default)]
    pub text: HashMap<String, LocalizedText>,
}

impl Exercise {
    pub fn solution_keys(&self) -> Vec<Vec<Key>> {
        self.solutions.iter().map(|s| parse_keys(s)).collect()
    }

    /// Решение, которому учит урок — первое в списке. Его показываем, когда
    /// строгий режим сообщает, чего он ждал.
    pub fn taught(&self) -> &str {
        self.solutions.first().map(|s| s.as_str()).unwrap_or("")
    }

    /// Самое короткое решение: именно оно считается оптимумом и задаёт звёзды.
    pub fn canonical(&self) -> &str {
        self.solutions
            .iter()
            .min_by_key(|s| parse_keys(s).len())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn optimal_keystrokes(&self) -> usize {
        self.solution_keys().iter().map(|k| k.len()).min().unwrap_or(1)
    }

    pub fn text(&self, lang: &str) -> LocalizedText {
        pick(&self.text, lang)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Lesson {
    pub id: String,
    pub chapter: String,
    #[serde(default)]
    pub order: usize,
    #[serde(default)]
    pub text: HashMap<String, LocalizedText>,
    #[serde(rename = "exercise")]
    pub exercises: Vec<Exercise>,
}

impl Lesson {
    pub fn text(&self, lang: &str) -> LocalizedText {
        pick(&self.text, lang)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Chapter {
    pub id: String,
    #[serde(default)]
    pub order: usize,
    #[serde(default)]
    pub text: HashMap<String, LocalizedText>,
    #[serde(default)]
    pub coming_soon: bool,
}

impl Chapter {
    pub fn text(&self, lang: &str) -> LocalizedText {
        pick(&self.text, lang)
    }
}

#[derive(Debug, Deserialize)]
struct ChaptersFile {
    #[serde(rename = "chapter")]
    chapters: Vec<Chapter>,
}

fn pick(map: &HashMap<String, LocalizedText>, lang: &str) -> LocalizedText {
    map.get(lang)
        .or_else(|| map.get("en"))
        .or_else(|| map.get("ru"))
        .cloned()
        .unwrap_or_else(LocalizedText::empty)
}

#[derive(Debug, Clone)]
pub struct Curriculum {
    pub chapters: Vec<Chapter>,
    pub lessons: Vec<Lesson>,
}

impl Curriculum {
    pub fn load() -> Self {
        let chapters_src = CONTENT
            .get_file("chapters.toml")
            .and_then(|f| f.contents_utf8())
            .expect("content/chapters.toml должен быть встроен в бинарь");
        let mut chapters: Vec<Chapter> = toml::from_str::<ChaptersFile>(chapters_src)
            .expect("chapters.toml не разобрался")
            .chapters;
        chapters.sort_by_key(|c| c.order);

        let mut lessons: Vec<Lesson> = Vec::new();
        collect_lessons(&CONTENT, &mut lessons);
        lessons.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));

        Self { chapters, lessons }
    }

    pub fn lessons_of(&self, chapter: &str) -> Vec<&Lesson> {
        self.lessons.iter().filter(|l| l.chapter == chapter).collect()
    }

    pub fn lesson(&self, id: &str) -> Option<&Lesson> {
        self.lessons.iter().find(|l| l.id == id)
    }

    pub fn chapter(&self, id: &str) -> Option<&Chapter> {
        self.chapters.iter().find(|c| c.id == id)
    }
}

fn collect_lessons(dir: &Dir<'_>, out: &mut Vec<Lesson>) {
    for file in dir.files() {
        let Some(name) = file.path().file_name().and_then(|n| n.to_str()) else { continue };
        if name == "chapters.toml" || !name.ends_with(".toml") {
            continue;
        }
        let src = file.contents_utf8().expect("урок должен быть в UTF-8");
        match toml::from_str::<Lesson>(src) {
            Ok(lesson) => out.push(lesson),
            Err(e) => panic!("урок {} не разобрался: {e}", file.path().display()),
        }
    }
    for sub in dir.dirs() {
        collect_lessons(sub, out);
    }
}
