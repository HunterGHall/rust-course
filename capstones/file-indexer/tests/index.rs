use std::fs;
use std::path::{Path, PathBuf};

use indexer::{FileIndex, SearchIndex, build_index, discover, tokenize};

/// A throwaway directory that cleans up after itself.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!("indexer-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    fn write(&self, name: &str, text: &str) {
        fs::write(self.0.join(name), text).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn tokenizer_lowercases_and_splits() {
    let words: Vec<String> = tokenize("Hello, World! snake_case 42x").collect();
    assert_eq!(words, vec!["hello", "world", "snake_case", "42x"]);
}

#[test]
fn counts_words() {
    let file = FileIndex::from_text(PathBuf::from("a.md"), "to be or not to be");
    assert_eq!(file.total_words, 6);
    assert_eq!(file.counts["to"], 2);
    assert_eq!(file.counts["or"], 1);
}

#[test]
fn search_requires_all_words_and_ranks_by_count() {
    let mut index = SearchIndex::new();
    index.add(FileIndex::from_text(
        "a.md".into(),
        "rust rust rust ownership",
    ));
    index.add(FileIndex::from_text(
        "b.md".into(),
        "rust ownership ownership",
    ));
    index.add(FileIndex::from_text("c.md".into(), "rust only"));

    let hits = index.search(&["rust", "ownership"]);
    let found: Vec<(&Path, u32)> = hits.iter().map(|h| (h.path.as_path(), h.score)).collect();
    assert_eq!(found, vec![(Path::new("a.md"), 4), (Path::new("b.md"), 3)]);

    assert!(index.search(&["missing"]).is_empty());
    assert!(index.search(&[]).is_empty());
}

#[test]
fn same_results_for_any_job_count() {
    let dir = TempDir::new("jobs");
    for i in 0..40 {
        let text = format!("common word{} {}{}", i % 7, "rust ".repeat(i % 5), i);
        dir.write(&format!("f{i:02}.md"), &text);
    }
    dir.write("ignored.txt", "common");

    let paths = discover(&dir.0, "md").unwrap();
    assert_eq!(paths.len(), 40);

    let one = build_index(&paths, 1);
    let eight = build_index(&paths, 8);
    assert!(one.errors.is_empty() && eight.errors.is_empty());
    assert_eq!(one.index.file_count(), 40);
    assert_eq!(one.index.total_words(), eight.index.total_words());
    assert_eq!(one.index.search(&["rust"]), eight.index.search(&["rust"]));
    assert_eq!(
        one.index.search(&["common", "word3"]),
        eight.index.search(&["common", "word3"])
    );
}

#[test]
fn unreadable_files_are_reported_not_fatal() {
    let dir = TempDir::new("errors");
    dir.write("good.md", "hello");
    let mut paths = discover(&dir.0, "md").unwrap();
    paths.push(dir.0.join("vanished.md"));

    let result = build_index(&paths, 4);
    assert_eq!(result.index.file_count(), 1);
    assert_eq!(result.errors.len(), 1);
}

#[test]
fn empty_input_is_fine() {
    let result = build_index(&[], 4);
    assert_eq!(result.index.file_count(), 0);
}
