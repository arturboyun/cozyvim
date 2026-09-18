//! Уроки встраиваются в бинарь через `include_dir!`, а cargo про этот каталог
//! ничего не знает: правка TOML без этого файла не вызывала пересборку, и
//! тесты гоняли старый контент.

use std::path::Path;

fn main() {
    println!("cargo::rerun-if-changed=content");
    watch(Path::new("content"));
}

fn watch(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        println!("cargo::rerun-if-changed={}", path.display());
        if path.is_dir() {
            watch(&path);
        }
    }
}
