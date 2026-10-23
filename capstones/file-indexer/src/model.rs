use std::collections::HashMap;
use std::path::PathBuf;

/// Lowercased alphanumeric words. Everything else separates words.
pub fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
}

/// Word counts for a single file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIndex {
    pub path: PathBuf,
    pub counts: HashMap<String, u32>,
    pub total_words: usize,
}

impl FileIndex {
    pub fn from_text(path: PathBuf, text: &str) -> FileIndex {
        let mut counts = HashMap::new();
        let mut total_words = 0;
        for word in tokenize(text) {
            *counts.entry(word).or_insert(0) += 1;
            total_words += 1;
        }
        FileIndex {
            path,
            counts,
            total_words,
        }
    }
}

/// One search result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    pub score: u32,
}

/// An inverted index: word -> which files contain it, and how often.
#[derive(Debug, Default)]
pub struct SearchIndex {
    files: Vec<FileIndex>,
    postings: HashMap<String, Vec<(usize, u32)>>,
}

impl SearchIndex {
    pub fn new() -> SearchIndex {
        SearchIndex::default()
    }

    pub fn add(&mut self, file: FileIndex) {
        let id = self.files.len();
        for (word, &count) in &file.counts {
            self.postings
                .entry(word.clone())
                .or_default()
                .push((id, count));
        }
        self.files.push(file);
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn word_count(&self) -> usize {
        self.postings.len()
    }

    pub fn total_words(&self) -> usize {
        self.files.iter().map(|f| f.total_words).sum()
    }

    /// Files containing *every* query word, best first (ties broken by path).
    pub fn search(&self, query: &[&str]) -> Vec<Hit> {
        let words: Vec<String> = query.iter().flat_map(|q| tokenize(q)).collect();
        if words.is_empty() {
            return Vec::new();
        }

        // Start from the first word's postings, then keep only files that also match the rest.
        let mut scores: HashMap<usize, u32> = HashMap::new();
        for (position, word) in words.iter().enumerate() {
            let Some(postings) = self.postings.get(word) else {
                return Vec::new();
            };
            if position == 0 {
                scores.extend(postings.iter().copied());
            } else {
                let here: HashMap<usize, u32> = postings.iter().copied().collect();
                scores.retain(|id, _| here.contains_key(id));
                for (id, score) in scores.iter_mut() {
                    *score += here[id];
                }
            }
        }

        let mut hits: Vec<Hit> = scores
            .into_iter()
            .map(|(id, score)| Hit {
                path: self.files[id].path.clone(),
                score,
            })
            .collect();
        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
        hits
    }
}
