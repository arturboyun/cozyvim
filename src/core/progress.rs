//! Прогресс считается поверх журнала: никакого отдельного состояния,
//! которое может разойтись с событиями.

use std::collections::HashMap;

use crate::content::Curriculum;
use crate::core::journal::Journal;

#[derive(Debug, Clone, Default)]
pub struct Progress {
    /// Лучшие звёзды: (id урока, номер упражнения) → звёзды.
    best: HashMap<(String, usize), u8>,
    attempts_per_exercise: HashMap<(String, usize), usize>,
    failures_per_command: HashMap<String, usize>,
    days_active: Vec<String>,
}

impl Progress {
    pub fn from_journal(journal: &Journal) -> Self {
        let mut p = Progress::default();
        for a in &journal.attempts {
            let key = (a.lesson_id.clone(), a.exercise_index);
            *p.attempts_per_exercise.entry(key.clone()).or_insert(0) += 1;
            if a.success {
                let slot = p.best.entry(key).or_insert(0);
                *slot = (*slot).max(a.stars);
            } else {
                *p.failures_per_command.entry(a.canonical.clone()).or_insert(0) += 1;
            }
            let day = a.ts.chars().take(10).collect::<String>();
            if p.days_active.last() != Some(&day) && !p.days_active.contains(&day) {
                p.days_active.push(day);
            }
        }
        p.days_active.sort();
        p
    }

    pub fn stars(&self, lesson_id: &str, exercise: usize) -> u8 {
        self.best.get(&(lesson_id.to_string(), exercise)).copied().unwrap_or(0)
    }

    pub fn lesson_stars(&self, curriculum: &Curriculum, lesson_id: &str) -> (u8, u8) {
        let Some(lesson) = curriculum.lesson(lesson_id) else { return (0, 0) };
        let earned: u32 =
            (0..lesson.exercises.len()).map(|i| self.stars(lesson_id, i) as u32).sum();
        let max = (lesson.exercises.len() * 3) as u32;
        (earned.min(255) as u8, max.min(255) as u8)
    }

    pub fn lesson_done(&self, curriculum: &Curriculum, lesson_id: &str) -> bool {
        let Some(lesson) = curriculum.lesson(lesson_id) else { return false };
        !lesson.exercises.is_empty()
            && (0..lesson.exercises.len()).all(|i| self.stars(lesson_id, i) > 0)
    }

    pub fn chapter_done(&self, curriculum: &Curriculum, chapter: &str) -> bool {
        let lessons = curriculum.lessons_of(chapter);
        !lessons.is_empty() && lessons.iter().all(|l| self.lesson_done(curriculum, &l.id))
    }

    /// Главы открываются последовательно; уроки внутри главы — все сразу.
    pub fn chapter_unlocked(&self, curriculum: &Curriculum, chapter: &str) -> bool {
        let idx = curriculum.chapters.iter().position(|c| c.id == chapter);
        match idx {
            None => false,
            Some(0) => true,
            Some(i) => {
                let prev = &curriculum.chapters[i - 1];
                self.chapter_done(curriculum, &prev.id)
            }
        }
    }

    pub fn total_stars(&self) -> u32 {
        self.best.values().map(|s| *s as u32).sum()
    }

    pub fn exercises_done(&self) -> usize {
        self.best.values().filter(|s| **s > 0).count()
    }

    /// Команды, на которых чаще всего спотыкаются — основа экрана «слабые места».
    pub fn problem_commands(&self, limit: usize) -> Vec<(String, usize)> {
        let mut v: Vec<(String, usize)> =
            self.failures_per_command.iter().map(|(k, c)| (k.clone(), *c)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(limit);
        v
    }

    pub fn days_active(&self) -> &[String] {
        &self.days_active
    }

    /// Череда дней подряд, считая от последнего активного дня.
    pub fn streak(&self) -> usize {
        use chrono::NaiveDate;
        let mut days: Vec<NaiveDate> = self
            .days_active
            .iter()
            .filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .collect();
        days.sort();
        days.dedup();
        let mut streak = 0;
        let mut expected: Option<NaiveDate> = None;
        for day in days.iter().rev() {
            match expected {
                None => {
                    streak = 1;
                    expected = day.pred_opt();
                }
                Some(e) if e == *day => {
                    streak += 1;
                    expected = day.pred_opt();
                }
                _ => break,
            }
        }
        streak
    }
}
