# 12. Capstone: command-line task tracker

## Goal

Build a real command-line app that stores data on disk between runs. It pulls together almost everything in Beginner: variables, strings, vectors, `match`, functions, ownership and borrowing, `Option`, `Result`, `?`, and file I/O. It also gives you a first taste of `struct`, `enum` and `impl`, which the Intermediate level covers properly.

## What it does

```
$ cargo run -q -- add Buy milk
Added #1: Buy milk
$ cargo run -q -- add Learn Rust
Added #2: Learn Rust
$ cargo run -q -- done 2
1. [ ] Buy milk
2. [x] Learn Rust
1/2 done
$ cargo run -q -- remove 9
error: no task #9 (you have 2)
$ echo $?
1
```

(`-q` silences Cargo's own build messages; everything after `--` goes to the program.)

## Requirements

1. Commands: `add <title>`, `list`, `done <n>`, `undo <n>`, `remove <n>`, `clear` (delete finished tasks).
2. Tasks are numbered from 1 in the display and persist to a text file (`tasks.txt`, or the path in the `TASKS_FILE` environment variable) after every change.
3. Bad input is reported with a friendly message on **standard error** and exit code `1` — never a panic.
4. A missing storage file means "no tasks yet", not an error.
5. A corrupt storage file is reported with the line number.

## Setup

```bash
cargo new tasks
cd tasks
```

Put the code below in `src/main.rs`. There are no dependencies — this is all standard library.

## The design

- **`TodoItem`** — a small `struct` bundling a title and a done flag. (A `struct` is a custom type with named fields; you'll study them next level.)
- **`TrackerError`** — one `enum` for everything that can go wrong, with a friendly message for each (`impl fmt::Display`).
- **Storage** (`load_tasks`, `save_tasks`) — a text format with one task per line, `0|title` or `1|title`. Simple, human-readable, easy to debug. `split_once('|')` means titles may themselves contain `|`.
- **Commands** — one `run` function with a `match` on the first argument.
- **Entry point** — `main` turns the `Result` from `run` into a message on stderr and an exit code.

Notice the layering: storage knows nothing about commands, and commands know nothing about the file format. Also notice how ownership shapes the signatures — `load_tasks` returns an owned `Vec<TodoItem>`, `save_tasks` and `print_list` only borrow it as `&[TodoItem]`.

## The full program

<!-- sync: capstones/task-tracker/src/main.rs -->
```rust
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
```
<!-- /sync -->

## Walkthrough

**Errors as data.** `TrackerError` has four variants. `NoSuchTask { number, count }` carries data, so the message can say "no task #9 (you have 2)". `impl fmt::Display` is what lets `println!("{error}")` work.

**`load_tasks`.** `fs::read_to_string` returns a `Result`. The `match` treats `ErrorKind::NotFound` as success with an empty list (rule 4) and everything else as a real error. Then each line is parsed with `let Some((flag, title)) = line.split_once('|') else { ... }` — `let else` keeps the happy path flat. `enumerate()` supplies line numbers for the corrupt-file message.

**`parse_number`.** User input like `"3"` becomes a zero-based index through `str::parse::<usize>()` and `map_err`, which converts the library's error into our friendlier `TrackerError::Usage`. Notice `usize` can't be negative, so `-1` is rejected by `parse` itself.

**`run`.** It loads the tasks, matches on the command, and mutates the `Vec` (`push`, `remove`, `retain`, or flipping `is_done` through `tasks[index]`). Each mutating command saves immediately. Every fallible call ends in `?`, so errors bubble up to one place.

**`main`.** Returning `ExitCode` lets us print to stderr and exit with status 1 without calling `std::process::exit`, which would skip destructors.

## Try it

```bash
cargo run -q -- add "Write the report"
cargo run -q -- add Call mum
cargo run -q -- list
cargo run -q -- done 1
cargo run -q -- clear
cargo run -q -- done abc          # error: 'abc' is not a task number
cargo run -q -- done 0            # error: no task #0 (you have 1)
TASKS_FILE=/tmp/other.txt cargo run -q -- list
echo "garbage" > tasks.txt
cargo run -q -- list              # error: storage file is corrupt at line 1
```

The complete, buildable project lives in [`capstones/task-tracker`](https://github.com/HunterGHall/rust-course/tree/main/capstones/task-tracker), and CI runs a smoke test of it on Linux, macOS and Windows.

## Self-check

- [ ] `cargo build` shows no warnings and `cargo clippy` is quiet.
- [ ] You can explain why `print_list` takes `&[TodoItem]` rather than `Vec<TodoItem>`.
- [ ] You can explain what would happen if `save_tasks` took `tasks: Vec<TodoItem>` (hint: the move).
- [ ] No `unwrap()` or `expect()` appears in the program.
- [ ] Running it with no arguments prints a usage message and exits with status 1.

## Stretch goals

1. **`edit <n> <new title>`** to rename a task.
2. **`list --pending`** to show only unfinished tasks (keep the original numbers in the output).
3. **Priorities:** add a `priority: u8` field, store it in the file (`0|2|title`), and sort by it.
4. **Due dates** as plain `YYYY-MM-DD` strings, with a `list --overdue` that compares them (strings in that format sort correctly).
5. **Atomic saves:** write to `tasks.txt.tmp` then `fs::rename` it over `tasks.txt`, so a crash can't corrupt the file.
6. **Undo the last change** by keeping a backup copy of the previous file.
7. **Unit tests:** move the parsing logic into functions that take `&str` and return `Result`, and test them with `#[test]` (you'll learn how in Intermediate lesson 11).

## When you're done

You can explain every line of the program and add a feature to it unaided. Then go to [Intermediate](../intermediate/index.md).
