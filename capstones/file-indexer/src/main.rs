use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Instant;

use indexer::{build_index, discover};

const USAGE: &str = "usage: indexer [--jobs N] [--ext EXT] [--top N] [--stats] <dir> <word>...";

struct Options {
    root: PathBuf,
    words: Vec<String>,
    jobs: usize,
    extension: String,
    top: usize,
    stats: bool,
}

fn parse_args() -> Result<Options, String> {
    let default_jobs = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let mut jobs = default_jobs;
    let mut extension = "md".to_string();
    let mut top = 10;
    let mut stats = false;
    let mut positional = Vec::new();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--jobs" => jobs = number(&arg, args.next())?,
            "--top" => top = number(&arg, args.next())?,
            "--ext" => extension = args.next().ok_or("--ext needs a value")?,
            "--stats" => stats = true,
            _ if arg.starts_with("--") => return Err(format!("unknown option {arg}")),
            _ => positional.push(arg),
        }
    }

    if positional.len() < 2 {
        return Err(USAGE.to_string());
    }
    let root = PathBuf::from(positional.remove(0));
    Ok(Options {
        root,
        words: positional,
        jobs,
        extension,
        top,
        stats,
    })
}

fn number(flag: &str, value: Option<String>) -> Result<usize, String> {
    let value = value.ok_or(format!("{flag} needs a value"))?;
    match value.parse::<usize>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(format!("{flag} needs a positive number, got '{value}'")),
    }
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(2);
        }
    };

    let paths = match discover(&options.root, &options.extension) {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", options.root.display());
            return ExitCode::from(2);
        }
    };

    let started = Instant::now();
    let result = build_index(&paths, options.jobs);
    let elapsed = started.elapsed();

    for (path, error) in &result.errors {
        eprintln!("warning: skipped {}: {error}", path.display());
    }

    let query: Vec<&str> = options.words.iter().map(String::as_str).collect();
    let hits = result.index.search(&query);
    for hit in hits.iter().take(options.top) {
        println!("{:>5}  {}", hit.score, hit.path.display());
    }
    if hits.is_empty() {
        println!("no matches");
    }

    if options.stats {
        // Stats go to stderr so stdout stays identical for any --jobs.
        eprintln!(
            "indexed {} files, {} words ({} distinct) in {:.1?} with {} job(s)",
            result.index.file_count(),
            result.index.total_words(),
            result.index.word_count(),
            elapsed,
            options.jobs
        );
    }
    if hits.is_empty() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
