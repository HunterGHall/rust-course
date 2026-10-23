use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

// ---------- Model ----------

struct TodoItem {
    title: String,
    is_done: bool,
}

// ---------- Errors ----------

enum TrackerError {
    Usage(String),
    NoSuchTask { number: usize, count: usize },
    Corrupt { line: usize },
    Io(String),
}

impl fmt::Display for TrackerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TrackerError::Usage(message) => write!(f, "{message}"),
            TrackerError::NoSuchTask { number, count } => {
                write!(f, "no task #{number} (you have {count})")
            }
            TrackerError::Corrupt { line } => {
                write!(f, "storage file is corrupt at line {line}")
            }
            TrackerError::Io(message) => write!(f, "{message}"),
        }
    }
}

// ---------- Storage ----------
//
// One task per line: `0|title` (open) or `1|title` (done).

fn storage_path() -> PathBuf {
    match env::var("TASKS_FILE") {
        Ok(path) => PathBuf::from(path),
        Err(_) => PathBuf::from("tasks.txt"),
    }
}

fn load_tasks(path: &PathBuf) -> Result<Vec<TodoItem>, TrackerError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        // A missing file just means "no tasks yet".
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(TrackerError::Io(format!(
                "cannot read {}: {e}",
                path.display()
            )));
        }
    };

    let mut tasks = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let Some((flag, title)) = line.split_once('|') else {
            return Err(TrackerError::Corrupt { line: index + 1 });
        };
        let is_done = match flag {
            "0" => false,
            "1" => true,
            _ => return Err(TrackerError::Corrupt { line: index + 1 }),
        };
        tasks.push(TodoItem {
            title: title.to_string(),
            is_done,
        });
    }
    Ok(tasks)
}

fn save_tasks(path: &PathBuf, tasks: &[TodoItem]) -> Result<(), TrackerError> {
    let mut text = String::new();
    for task in tasks {
        let flag = if task.is_done { '1' } else { '0' };
        text.push(flag);
        text.push('|');
        text.push_str(&task.title);
        text.push('\n');
    }
    fs::write(path, text)
        .map_err(|e| TrackerError::Io(format!("cannot write {}: {e}", path.display())))
}

// ---------- Commands ----------

fn print_list(tasks: &[TodoItem]) {
    if tasks.is_empty() {
        println!("no tasks yet");
        return;
    }
    for (index, task) in tasks.iter().enumerate() {
        let mark = if task.is_done { "x" } else { " " };
        println!("{}. [{}] {}", index + 1, mark, task.title);
    }
    let done = tasks.iter().filter(|t| t.is_done).count();
    println!("{}/{} done", done, tasks.len());
}

/// Turns the "3" in `tasks done 3` into a zero-based index.
fn parse_number(args: &[String], count: usize) -> Result<usize, TrackerError> {
    let Some(text) = args.first() else {
        return Err(TrackerError::Usage("expected a task number".to_string()));
    };
    let number: usize = text
        .parse()
        .map_err(|_| TrackerError::Usage(format!("'{text}' is not a task number")))?;
    if number == 0 || number > count {
        return Err(TrackerError::NoSuchTask { number, count });
    }
    Ok(number - 1)
}

fn run(args: &[String]) -> Result<(), TrackerError> {
    let path = storage_path();
    let mut tasks = load_tasks(&path)?;

    let Some(command) = args.first() else {
        return Err(TrackerError::Usage(
            "usage: tasks add <title> | list | done <n> | undo <n> | remove <n> | clear"
                .to_string(),
        ));
    };
    let rest = &args[1..];

    match command.as_str() {
        "add" => {
            let title = rest.join(" ");
            if title.trim().is_empty() {
                return Err(TrackerError::Usage("add needs a title".to_string()));
            }
            tasks.push(TodoItem {
                title: title.clone(),
                is_done: false,
            });
            save_tasks(&path, &tasks)?;
            println!("Added #{}: {}", tasks.len(), title);
        }
        "list" => print_list(&tasks),
        "done" | "undo" => {
            let index = parse_number(rest, tasks.len())?;
            tasks[index].is_done = command == "done";
            save_tasks(&path, &tasks)?;
            print_list(&tasks);
        }
        "remove" => {
            let index = parse_number(rest, tasks.len())?;
            let removed = tasks.remove(index);
            save_tasks(&path, &tasks)?;
            println!("Removed: {}", removed.title);
        }
        "clear" => {
            tasks.retain(|t| !t.is_done);
            save_tasks(&path, &tasks)?;
            print_list(&tasks);
        }
        other => {
            return Err(TrackerError::Usage(format!("unknown command '{other}'")));
        }
    }
    Ok(())
}

// ---------- Entry point ----------

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
