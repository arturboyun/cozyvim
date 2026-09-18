//! Каждый урок — сам себе тест: прогоняем все заявленные решения через движок
//! и сверяем с `goal`. Ловит и баги движка, и опечатки в контенте.

use cozyvim::content::Curriculum;
use cozyvim::core::buffer::Buffer;
use cozyvim::core::keys::parse_keys;
use cozyvim::vim::engine::{Engine, Mode};

#[test]
fn every_solution_reaches_its_goal() {
    let curriculum = Curriculum::load();
    let mut failures: Vec<String> = Vec::new();

    for lesson in &curriculum.lessons {
        for (idx, ex) in lesson.exercises.iter().enumerate() {
            assert!(
                !ex.solutions.is_empty(),
                "{} #{idx}: у упражнения нет ни одного решения",
                lesson.id
            );
            for solution in &ex.solutions {
                let mut buf = Buffer::new(&ex.buffer);
                buf.cursor = (ex.cursor[0], ex.cursor[1]);
                let mut eng = Engine::new(buf);
                for k in parse_keys(solution) {
                    eng.feed(k);
                }
                if eng.mode != Mode::Normal {
                    failures.push(format!(
                        "{} #{idx} «{solution}»: решение оставляет редактор в режиме {:?}",
                        lesson.id, eng.mode
                    ));
                    continue;
                }
                let got = eng.buf.to_lines();
                if got != ex.goal {
                    failures.push(format!(
                        "{} #{idx} «{solution}»: получилось {:?}, ожидалось {:?}",
                        lesson.id, got, ex.goal
                    ));
                }
                if let Some([row, col]) = ex.goal_cursor {
                    if eng.buf.cursor != (row, col) {
                        failures.push(format!(
                            "{} #{idx} «{solution}»: курсор {:?}, ожидался {:?}",
                            lesson.id,
                            eng.buf.cursor,
                            (row, col)
                        ));
                    }
                }
            }
        }
    }

    assert!(failures.is_empty(), "проблемы в уроках:\n{}", failures.join("\n"));
}

#[test]
fn solutions_are_distinct_and_canonical_is_shortest() {
    let curriculum = Curriculum::load();
    for lesson in &curriculum.lessons {
        for (idx, ex) in lesson.exercises.iter().enumerate() {
            let mut seen = ex.solutions.clone();
            seen.sort();
            seen.dedup();
            assert_eq!(
                seen.len(),
                ex.solutions.len(),
                "{} #{idx}: одно и то же решение перечислено дважды",
                lesson.id
            );
            let shortest = ex.solution_keys().iter().map(|k| k.len()).min().unwrap();
            assert_eq!(
                parse_keys(ex.canonical()).len(),
                shortest,
                "{} #{idx}: оптимум выбран не по длине",
                lesson.id
            );
        }
    }
}

#[test]
fn lessons_belong_to_declared_chapters() {
    let curriculum = Curriculum::load();
    for lesson in &curriculum.lessons {
        assert!(
            curriculum.chapter(&lesson.chapter).is_some(),
            "урок {} ссылается на несуществующую главу {}",
            lesson.id,
            lesson.chapter
        );
        assert!(!lesson.exercises.is_empty(), "урок {} пустой", lesson.id);
        for lang in ["ru", "en"] {
            assert!(!lesson.text(lang).title.is_empty(), "у урока {} нет названия ({lang})", lesson.id);
            for (i, ex) in lesson.exercises.iter().enumerate() {
                assert!(
                    !ex.text(lang).task.is_empty(),
                    "у упражнения {} #{i} нет задания ({lang})",
                    lesson.id
                );
            }
        }
    }
}
