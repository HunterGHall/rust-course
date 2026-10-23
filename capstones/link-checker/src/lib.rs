//! A tiny Markdown link checker: a testable library behind a thin CLI.

pub mod checker;
pub mod links;

pub use checker::{
    FileReport, LinkReport, Summary, Verdict, check_file, find_markdown, render, summarize,
};
pub use links::{Link, LinkKind, classify, extract_links, heading_slugs, slugify};
