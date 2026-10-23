use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use linkcheck::{check_file, find_markdown, render, summarize};

fn main() -> ExitCode {
    let mut roots: Vec<PathBuf> = env::args().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        roots.push(PathBuf::from("."));
    }

    let mut reports = Vec::new();
    for root in &roots {
        let files = match find_markdown(root) {
            Ok(files) => files,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", root.display());
                return ExitCode::from(2);
            }
        };
        for file in files {
            match check_file(&file) {
                Ok(report) => reports.push(report),
                Err(e) => {
                    eprintln!("error: cannot read {}: {e}", file.display());
                    return ExitCode::from(2);
                }
            }
        }
    }

    print!("{}", render(&reports));
    if summarize(&reports).broken > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
