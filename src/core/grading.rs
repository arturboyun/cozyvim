//! Оценка попытки. Три режима проверки из настроек отличаются только тем,
//! что считается успехом; звёзды всегда про эффективность.

use crate::content::Exercise;
use crate::core::keys::{Key, keys_notation};
use crate::core::settings::CheckMode;

#[derive(Debug, Clone, PartialEq)]
pub enum AttemptState {
    InProgress,
    Success { stars: u8 },
    /// Строгий режим: последовательность разошлась с любым из решений.
    Failed { expected: String },
}

/// Звёзды за эффективность: эталон — три, полтора эталона — две, иначе одна.
pub fn stars_for(keystrokes: usize, optimal: usize) -> u8 {
    if keystrokes <= optimal {
        3
    } else if keystrokes as f32 <= optimal as f32 * 1.5 {
        2
    } else {
        1
    }
}

pub fn evaluate(
    mode: CheckMode,
    pressed: &[Key],
    exercise: &Exercise,
    buffer: &[String],
    cursor: (usize, usize),
    in_normal_mode: bool,
) -> AttemptState {
    let solutions = exercise.solution_keys();
    match mode {
        CheckMode::Strict => {
            if solutions.iter().any(|s| s == pressed) {
                return AttemptState::Success { stars: 3 };
            }
            let still_possible = solutions
                .iter()
                .any(|s| s.len() > pressed.len() && s[..pressed.len()] == *pressed);
            if still_possible {
                AttemptState::InProgress
            } else {
                AttemptState::Failed { expected: exercise.canonical().to_string() }
            }
        }
        CheckMode::Mix | CheckMode::Free => {
            let cursor_ok = match exercise.goal_cursor {
                Some([row, col]) => cursor == (row, col),
                None => true,
            };
            let reached = buffer == exercise.goal && cursor_ok && in_normal_mode;
            if !reached {
                return AttemptState::InProgress;
            }
            let stars = match mode {
                CheckMode::Free => 3,
                _ => stars_for(pressed.len(), exercise.optimal_keystrokes()),
            };
            AttemptState::Success { stars }
        }
    }
}

pub fn pressed_notation(pressed: &[Key]) -> String {
    keys_notation(pressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::keys::parse_keys;
    use std::collections::HashMap;

    fn exercise() -> Exercise {
        Exercise {
            buffer: vec!["let foo = 1;".into()],
            cursor: [0, 4],
            goal: vec!["let  = 1;".into()],
            goal_cursor: None,
            solutions: vec!["dw".into(), "daw".into()],
            text: HashMap::new(),
        }
    }

    #[test]
    fn strict_accepts_any_listed_solution() {
        let ex = exercise();
        for s in ["dw", "daw"] {
            assert_eq!(
                evaluate(CheckMode::Strict, &parse_keys(s), &ex, &[], (0, 0), true),
                AttemptState::Success { stars: 3 }
            );
        }
    }

    #[test]
    fn strict_fails_on_divergence() {
        let ex = exercise();
        assert!(matches!(
            evaluate(CheckMode::Strict, &parse_keys("dj"), &ex, &[], (0, 0), true),
            AttemptState::Failed { .. }
        ));
        assert_eq!(
            evaluate(CheckMode::Strict, &parse_keys("d"), &ex, &[], (0, 0), true),
            AttemptState::InProgress
        );
    }

    #[test]
    fn mix_counts_stars_by_efficiency() {
        let ex = exercise();
        let goal: Vec<String> = ex.goal.clone();
        assert_eq!(
            evaluate(CheckMode::Mix, &parse_keys("dw"), &ex, &goal, (0, 0), true),
            AttemptState::Success { stars: 3 }
        );
        assert_eq!(
            evaluate(CheckMode::Mix, &parse_keys("xxx"), &ex, &goal, (0, 0), true),
            AttemptState::Success { stars: 2 }
        );
        assert_eq!(
            evaluate(CheckMode::Mix, &parse_keys("xxxxxxx"), &ex, &goal, (0, 0), true),
            AttemptState::Success { stars: 1 }
        );
    }

    #[test]
    fn insert_mode_does_not_count_as_finished() {
        let ex = exercise();
        let goal: Vec<String> = ex.goal.clone();
        assert_eq!(
            evaluate(CheckMode::Mix, &parse_keys("dw"), &ex, &goal, (0, 0), false),
            AttemptState::InProgress
        );
    }
}
