//! cozyvim — уютный тренажёр vim.

#[macro_use]
extern crate rust_i18n;

i18n!("locales", fallback = "en");

pub mod app;
pub mod content;
pub mod core;
pub mod ui;
pub mod vim;
