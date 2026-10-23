use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::links::{LinkKind, classify, extract_links, heading_slugs};

/// The result of checking one link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    Broken(String),
    Skipped(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkReport {
    pub line: usize,
    pub target: String,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReport {
    pub path: PathBuf,
    pub links: Vec<LinkReport>,
}

/// Every `.md` file under `root` (or `root` itself if it is a file), sorted.
pub fn find_markdown(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    collect(root, &mut found)?;
    found.sort();
    Ok(found)
}

fn collect(path: &Path, found: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // Skip hidden folders and build output.
            if child.is_dir()
                && (name.starts_with('.') || name == "target" || name == "node_modules")
            {
                continue;
            }
            collect(&child, found)?;
        }
    } else if path.extension().is_some_and(|e| e == "md") {
        found.push(path.to_path_buf());
    }
    Ok(())
}

/// Checks every link in one Markdown file.
pub fn check_file(path: &Path) -> io::Result<FileReport> {
    let text = fs::read_to_string(path)?;
    let own_slugs = heading_slugs(&text);
    let folder = path.parent().unwrap_or(Path::new("."));

    let mut links = Vec::new();
    for link in extract_links(&text) {
        let verdict = check_link(&link.target, folder, &own_slugs);
        links.push(LinkReport {
            line: link.line,
            target: link.target,
            verdict,
        });
    }
    Ok(FileReport {
        path: path.to_path_buf(),
        links,
    })
}

fn check_link(target: &str, folder: &Path, own_slugs: &[String]) -> Verdict {
    match classify(target) {
        LinkKind::Url => Verdict::Skipped("external URL".to_string()),
        LinkKind::Anchor(anchor) => {
            if own_slugs.contains(&anchor) {
                Verdict::Ok
            } else {
                Verdict::Broken(format!("no heading #{anchor} in this file"))
            }
        }
        LinkKind::File { path, fragment } => {
            let resolved = if path.is_empty() {
                folder.to_path_buf()
            } else {
                folder.join(&path)
            };
            if !resolved.exists() {
                return Verdict::Broken("file not found".to_string());
            }
            let Some(fragment) = fragment else {
                return Verdict::Ok;
            };
            if resolved.is_dir() || resolved.extension().is_none_or(|e| e != "md") {
                return Verdict::Ok; // anchors are only checkable in Markdown
            }
            match fs::read_to_string(&resolved) {
                Ok(text) if heading_slugs(&text).contains(&fragment) => Verdict::Ok,
                Ok(_) => Verdict::Broken(format!("no heading #{fragment} in {path}")),
                Err(e) => Verdict::Broken(format!("cannot read {path}: {e}")),
            }
        }
    }
}

// ---------- Reporting ----------

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub files: usize,
    pub ok: usize,
    pub broken: usize,
    pub skipped: usize,
}

impl Summary {
    pub fn total(&self) -> usize {
        self.ok + self.broken + self.skipped
    }
}

pub fn summarize(reports: &[FileReport]) -> Summary {
    let mut summary = Summary {
        files: reports.len(),
        ..Summary::default()
    };
    for report in reports {
        for link in &report.links {
            match link.verdict {
                Verdict::Ok => summary.ok += 1,
                Verdict::Broken(_) => summary.broken += 1,
                Verdict::Skipped(_) => summary.skipped += 1,
            }
        }
    }
    summary
}

/// Renders reports in the format shown in the lesson.
pub fn render(reports: &[FileReport]) -> String {
    let mut out = String::new();
    for report in reports {
        if report.links.is_empty() {
            continue;
        }
        out.push_str(&format!("\n{}\n", report.path.display()));
        for link in &report.links {
            let (label, note) = match &link.verdict {
                Verdict::Ok => ("OK   ", String::new()),
                Verdict::Broken(why) => ("FAIL ", format!("  ({why})")),
                Verdict::Skipped(why) => ("SKIP ", format!("  ({why})")),
            };
            out.push_str(&format!("  {label} {}:{}{note}\n", link.line, link.target));
        }
    }
    let s = summarize(reports);
    out.push_str(&format!(
        "\nChecked {} links in {} files: {} OK, {} broken, {} skipped\n",
        s.total(),
        s.files,
        s.ok,
        s.broken,
        s.skipped
    ));
    out
}
