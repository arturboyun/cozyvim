pub mod keyboard;
pub mod lesson;
pub mod map;
pub mod onboarding;
pub mod settings_screen;
pub mod stats;
pub mod theme;
pub mod widgets;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Onboarding { step: usize },
    Map,
    Lesson { lesson_id: String },
    Stats,
    Settings,
}
