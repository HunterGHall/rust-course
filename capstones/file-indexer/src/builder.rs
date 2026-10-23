use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use crate::model::{FileIndex, SearchIndex};

/// Every file under `root` with the given extension (no dot), sorted by path.
pub fn discover(root: &Path, extension: &str) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    walk(root, extension, &mut found)?;
    found.sort();
    Ok(found)
}

fn walk(path: &Path, extension: &str, found: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if child.is_dir() && (name.starts_with('.') || name == "target") {
                continue;
            }
            walk(&child, extension, found)?;
        }
    } else if path.extension().is_some_and(|e| e == extension) {
        found.push(path.to_path_buf());
    }
    Ok(())
}

/// Reads and tokenizes one file. Invalid UTF-8 is replaced, not rejected.
pub fn index_file(path: &Path) -> io::Result<FileIndex> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(FileIndex::from_text(path.to_path_buf(), &text))
}

pub struct BuildResult {
    pub index: SearchIndex,
    pub errors: Vec<(PathBuf, io::Error)>,
}

/// Indexes `paths` using up to `jobs` worker threads.
///
/// Workers only read files and send results back over a channel; the calling
/// thread is the only one that touches the `SearchIndex`, and it adds files in
/// path order, so the result is identical for any `jobs`.
pub fn build_index(paths: &[PathBuf], jobs: usize) -> BuildResult {
    let jobs = jobs.clamp(1, paths.len().max(1));
    let next = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel::<(usize, io::Result<FileIndex>)>();

    let mut slots: Vec<Option<io::Result<FileIndex>>> = Vec::new();
    slots.resize_with(paths.len(), || None);

    thread::scope(|scope| {
        for _ in 0..jobs {
            let sender = sender.clone();
            let next = &next;
            scope.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = paths.get(i) else { break };
                    if sender.send((i, index_file(path))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender); // so the receiver stops when the workers finish

        for (i, result) in receiver {
            slots[i] = Some(result);
        }
    });

    let mut index = SearchIndex::new();
    let mut errors = Vec::new();
    for (path, slot) in paths.iter().zip(slots) {
        match slot.expect("every path produces exactly one result") {
            Ok(file) => index.add(file),
            Err(e) => errors.push((path.clone(), e)),
        }
    }
    BuildResult { index, errors }
}
