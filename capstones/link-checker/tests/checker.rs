use std::fs;
use std::path::PathBuf;

use linkcheck::{Verdict, check_file, find_markdown, summarize};

/// A throwaway directory that cleans up after itself.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!("linkcheck-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn verdicts_for_each_kind_of_link() {
    let dir = TempDir::new("verdicts");
    dir.write("other.md", "# Install\n");
    let index = dir.write(
        "index.md",
        "# Top\n[ok](other.md) [missing](nope.md) [anchor](other.md#install) \
         [bad](other.md#instal) [self](#top) [ext](https://rust-lang.org)\n",
    );

    let report = check_file(&index).unwrap();
    let verdicts: Vec<&Verdict> = report.links.iter().map(|l| &l.verdict).collect();

    assert_eq!(verdicts[0], &Verdict::Ok);
    assert!(matches!(verdicts[1], Verdict::Broken(why) if why == "file not found"));
    assert_eq!(verdicts[2], &Verdict::Ok);
    assert!(matches!(verdicts[3], Verdict::Broken(_)));
    assert_eq!(verdicts[4], &Verdict::Ok);
    assert!(matches!(verdicts[5], Verdict::Skipped(_)));

    let summary = summarize(&[report]);
    assert_eq!((summary.ok, summary.broken, summary.skipped), (3, 2, 1));
}

#[test]
fn finds_markdown_recursively_and_sorted() {
    let dir = TempDir::new("walk");
    fs::create_dir_all(dir.0.join("sub")).unwrap();
    fs::create_dir_all(dir.0.join(".hidden")).unwrap();
    dir.write("b.md", "");
    dir.write("a.md", "");
    dir.write("notes.txt", "");
    fs::write(dir.0.join("sub").join("c.md"), "").unwrap();
    fs::write(dir.0.join(".hidden").join("d.md"), "").unwrap();

    let names: Vec<String> = find_markdown(&dir.0)
        .unwrap()
        .iter()
        .map(|p| {
            p.strip_prefix(&dir.0)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    assert_eq!(names, vec!["a.md", "b.md", "sub/c.md"]);
}
