//! Табличные тесты движка: (буфер, курсор, нажатия) → (буфер, курсор).

use cozyvim::core::buffer::Buffer;
use cozyvim::core::keys::parse_keys;
use cozyvim::vim::engine::{Engine, Mode};

fn run(lines: &[&str], cursor: (usize, usize), keys: &str) -> (Vec<String>, (usize, usize), Mode) {
    let mut buf = Buffer::from_str_slice(lines);
    buf.cursor = cursor;
    let mut eng = Engine::new(buf);
    for k in parse_keys(keys) {
        eng.feed(k);
    }
    (eng.buf.to_lines(), eng.buf.cursor, eng.mode)
}

fn check(lines: &[&str], cursor: (usize, usize), keys: &str, expect: &[&str]) {
    let (got, _, _) = run(lines, cursor, keys);
    let want: Vec<String> = expect.iter().map(|s| s.to_string()).collect();
    assert_eq!(got, want, "нажатия `{keys}` дали не тот буфер");
}

fn check_cursor(lines: &[&str], cursor: (usize, usize), keys: &str, expect: (usize, usize)) {
    let (_, got, _) = run(lines, cursor, keys);
    assert_eq!(got, expect, "нажатия `{keys}` увели курсор не туда");
}

#[test]
fn motions_move_cursor() {
    let text = ["fn main() {", "    let foo_bar = 42;", "}"];
    check_cursor(&text, (0, 0), "l", (0, 1));
    check_cursor(&text, (0, 0), "j", (1, 0));
    check_cursor(&text, (1, 0), "k", (0, 0));
    check_cursor(&text, (0, 0), "$", (0, 10));
    check_cursor(&text, (0, 5), "0", (0, 0));
    check_cursor(&text, (1, 0), "^", (1, 4));
    check_cursor(&text, (0, 0), "w", (0, 3));
    check_cursor(&text, (0, 3), "b", (0, 0));
    check_cursor(&text, (0, 0), "e", (0, 1));
    check_cursor(&text, (0, 0), "G", (2, 0));
    check_cursor(&text, (2, 0), "gg", (0, 0));
    check_cursor(&text, (0, 0), "3l", (0, 3));
}

#[test]
fn find_motions() {
    let text = ["let foo = bar;"];
    check_cursor(&text, (0, 0), "f=", (0, 8));
    check_cursor(&text, (0, 0), "t=", (0, 7));
    check_cursor(&text, (0, 13), "F=", (0, 8));
    check_cursor(&text, (0, 0), "f=;", (0, 8));
    check_cursor(&text, (0, 0), "fo;", (0, 6));
}

#[test]
fn delete_with_motion() {
    check(&["let foo = 42;"], (0, 4), "dw", &["let = 42;"]);
    check(&["let foo = 42;"], (0, 4), "de", &["let  = 42;"]);
    check(&["let foo = 42;"], (0, 4), "d$", &["let "]);
    check(&["let foo = 42;"], (0, 4), "x", &["let oo = 42;"]);
    check(&["let foo = 42;"], (0, 4), "3x", &["let  = 42;"]);
    check(&["let foo = 42;"], (0, 4), "D", &["let "]);
}

#[test]
fn delete_lines() {
    let text = ["one", "two", "three"];
    check(&text, (1, 0), "dd", &["one", "three"]);
    check(&text, (0, 0), "2dd", &["three"]);
    check(&text, (0, 0), "dj", &["three"]);
    check(&text, (0, 0), "dG", &[""]);
}

#[test]
fn change_commands() {
    let (lines, _, mode) = run(&["let foo = 42;"], (0, 4), "cw");
    assert_eq!(lines, vec!["let  = 42;".to_string()]);
    assert_eq!(mode, Mode::Insert);
    check(&["let foo = 42;"], (0, 4), "cwbar<Esc>", &["let bar = 42;"]);
    check(&["let foo = 42;"], (0, 4), "ciwbar<Esc>", &["let bar = 42;"]);
    check(&["one", "two"], (0, 0), "ccuno<Esc>", &["uno", "two"]);
}

#[test]
fn insert_commands() {
    check(&["bar"], (0, 0), "ifoo<Esc>", &["foobar"]);
    check(&["bar"], (0, 0), "abaz<Esc>", &["bbazar"]);
    check(&["    bar"], (0, 6), "Ifoo<Esc>", &["    foobar"]);
    check(&["bar"], (0, 0), "A!<Esc>", &["bar!"]);
    check(&["    bar"], (0, 0), "obaz<Esc>", &["    bar", "    baz"]);
    check(&["bar"], (0, 0), "Obaz<Esc>", &["baz", "bar"]);
}

#[test]
fn text_objects() {
    check(&["say \"hello world\" now"], (0, 8), "di\"", &["say \"\" now"]);
    check(&["say \"hello world\" now"], (0, 8), "da\"", &["say  now"]);
    check(&["call(a, b)"], (0, 6), "di(", &["call()"]);
    check(&["let foo_bar = 1;"], (0, 5), "diw", &["let  = 1;"]);
    check(&["let foo_bar = 1;"], (0, 5), "daw", &["let = 1;"]);
    check(&["a", "b", "", "c"], (0, 0), "dip", &["", "c"]);
}

#[test]
fn yank_and_put() {
    check(&["one", "two"], (0, 0), "yyp", &["one", "one", "two"]);
    check(&["one", "two"], (0, 0), "yyP", &["one", "one", "two"]);
    check(&["abc"], (0, 0), "ylp", &["aabc"]);
    check(&["one", "two"], (0, 0), "ddp", &["two", "one"]);
}

#[test]
fn undo_redo() {
    check(&["one"], (0, 0), "ddu", &["one"]);
    check(&["one", "two"], (0, 0), "ddu<C-r>", &["two"]);
    check(&["abc"], (0, 0), "xxu", &["bc"]);
}

#[test]
fn visual_mode() {
    check(&["hello world"], (0, 0), "vlld", &["lo world"]);
    check(&["one", "two", "three"], (0, 0), "Vjd", &["three"]);
    check(&["hello"], (0, 0), "vecya<Esc>", &["ya"]);
}

#[test]
fn replace_and_join() {
    check(&["cat"], (0, 0), "rb", &["bat"]);
    check(&["aaa"], (0, 0), "3rb", &["bbb"]);
    check(&["one", "two"], (0, 0), "J", &["one two"]);
}

#[test]
fn search_motions() {
    let text = ["alpha", "beta", "gamma beta"];
    check_cursor(&text, (0, 0), "/beta<CR>", (1, 0));
    check_cursor(&text, (0, 0), "/beta<CR>n", (2, 6));
    check_cursor(&text, (1, 0), "*", (2, 6));
}

#[test]
fn dot_repeats_last_change() {
    check(&["a a a a"], (0, 0), "dw.", &["a a"]);
    check(&["one two three"], (0, 0), "cwX<Esc>w.", &["X X three"]);
}

#[test]
fn counts_combine_with_operators() {
    check(&["a b c d e"], (0, 0), "d3w", &["d e"]);
    check(&["one", "two", "three", "four"], (0, 0), "2dd", &["three", "four"]);
}

#[test]
fn unsupported_commands_are_reported() {
    use cozyvim::core::keys::Key;
    use cozyvim::vim::engine::StepResult;
    let mut eng = Engine::new(Buffer::from_str_slice(&["abc"]));
    assert!(matches!(eng.feed(Key::Char(':')), StepResult::Unsupported(_)));
    assert!(matches!(eng.feed(Key::Char('q')), StepResult::Unsupported(_)));
    assert!(matches!(eng.feed(Key::Ctrl('v')), StepResult::Unsupported(_)));
}
