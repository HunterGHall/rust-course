# 7. Smart pointers

## What & why

A reference (`&T`) only borrows. Sometimes you need more: heap allocation of a single value, a value with *several* owners, or mutation that is checked at run time instead of compile time. The standard library provides **smart pointers** for each — types that own or manage data and implement `Deref` (to be used like a reference) and `Drop` (to clean up).

| Type | Gives you | Cost |
| --- | --- | --- |
| `Box<T>` | one owner, value on the heap | an allocation |
| `Rc<T>` | many owners (single-threaded) | a counter increment |
| `Arc<T>` | many owners (thread-safe) | an atomic counter increment |
| `RefCell<T>` | mutation through `&T`, checked at run time | a borrow flag |
| `Cell<T>` | mutation through `&T` for `Copy` values | none |

## `Box<T>`

```rust
fn main() {
    let b = Box::new(5);               // the integer lives on the heap
    println!("{b} {}", *b + 1);        // * dereferences it
}
```

Putting an `i32` on the heap is pointless, but there are three real uses.

**1. Recursive types** whose size would otherwise be infinite:

```rust
#[derive(Debug)]
enum List {
    Cons(i32, Box<List>),
    Nil,
}

use List::{Cons, Nil};

fn sum(list: &List) -> i32 {
    match list {
        Cons(value, rest) => value + sum(rest),
        Nil => 0,
    }
}

fn main() {
    let list = Cons(1, Box::new(Cons(2, Box::new(Cons(3, Box::new(Nil))))));
    println!("{}", sum(&list));        // 6
}
```

**2. Trait objects**: `Box<dyn Trait>` (lesson 3).

**3. Moving big values cheaply** — moving a `Box` copies only the pointer.

## `Deref` and `Drop`

```rust
use std::ops::Deref;

struct Wrapper(String);

impl Deref for Wrapper {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl Drop for Wrapper {
    fn drop(&mut self) {
        println!("dropping {}", self.0);
    }
}

fn shout(s: &str) {
    println!("{}!", s.to_uppercase());
}

fn main() {
    let w = Wrapper(String::from("hello"));
    shout(&w);                // &Wrapper -> &String -> &str: "deref coercion" chains automatically
    println!("{}", w.len());  // methods of String are available directly
    let early = Wrapper(String::from("early"));
    drop(early);              // runs Drop now ("dropping early")
    println!("end of main");  // then `w` is dropped at the closing brace
}
```

`Drop` is Rust's destructor: it runs automatically when an owner leaves scope, in reverse order of declaration. That's RAII — files close, locks release, memory frees — with no manual cleanup calls. You can't call `.drop()` directly; use `std::mem::drop(value)`.

## `Rc<T>`: shared ownership

Sometimes a value really has several owners (nodes in a graph, a config shared by components). `Rc` ("reference counted") keeps a count of owners and frees the value when the last one goes away:

```rust
use std::rc::Rc;

fn main() {
    let shared = Rc::new(String::from("shared data"));
    println!("{}", Rc::strong_count(&shared));       // 1

    let a = Rc::clone(&shared);                      // cheap: bumps the count, no deep copy
    {
        let b = Rc::clone(&shared);
        println!("{} {}", Rc::strong_count(&shared), b);   // 3
    }                                                // b dropped
    println!("{}", Rc::strong_count(&shared));       // 2
    println!("{} {}", a, shared.len());
}
```

`Rc<T>` gives read-only access (it's shared, after all). It is **not** thread-safe; for threads use `Arc` (Advanced lesson 5).

## `RefCell<T>`: interior mutability

Normally the borrow rules are enforced at compile time. `RefCell` moves them to **run time**, which lets you mutate data through a shared reference when you can prove it's safe but the compiler can't:

```rust
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(vec![1, 2, 3]);

    cell.borrow_mut().push(4);              // a temporary mutable borrow
    println!("{:?}", cell.borrow());        // [1, 2, 3, 4]
    println!("{}", cell.borrow().len());
}
```

If you break the rules, you get a **panic** instead of a compile error:

<!-- check: wrap panics -->
```rust
let cell = std::cell::RefCell::new(5);
let first = cell.borrow_mut();
let second = cell.borrow_mut();      // panics: already borrowed (BorrowMutError)
println!("{first} {second}");
```

`try_borrow` / `try_borrow_mut` return `Result`s instead of panicking. `Cell<T>` is the lighter cousin for `Copy` types:

```rust
use std::cell::Cell;

struct Counter {
    hits: Cell<u32>,
}

impl Counter {
    fn hit(&self) {                    // note: &self, not &mut self
        self.hits.set(self.hits.get() + 1);
    }
}

fn main() {
    let c = Counter { hits: Cell::new(0) };
    c.hit();
    c.hit();
    println!("{}", c.hits.get());      // 2
}
```

## `Rc<RefCell<T>>`: shared and mutable

Combine them for "many owners, each can mutate":

```rust
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct Account {
    balance: i64,
}

fn main() {
    let account = Rc::new(RefCell::new(Account { balance: 100 }));

    let for_alice = Rc::clone(&account);
    let for_bob = Rc::clone(&account);

    for_alice.borrow_mut().balance -= 30;
    for_bob.borrow_mut().balance += 5;

    println!("{:?}", account.borrow());          // Account { balance: 75 }
}
```

This is the Rust equivalent of ordinary references in garbage-collected languages — and you pay for it with run-time checks. Use it sparingly; most programs don't need it. Prefer plain ownership, passing `&mut`, or indexes into a `Vec`.

## `Weak<T>` and reference cycles

Two `Rc`s that point at each other never reach count zero: a memory leak (Rust considers leaks *safe*, though bad). A `Weak<T>` is a non-owning pointer that doesn't keep the value alive — ideal for parent links:

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    name: String,
    parent: RefCell<Weak<Node>>,          // weak: children don't keep parents alive
    children: RefCell<Vec<Rc<Node>>>,     // strong: parents own children
}

fn main() {
    let root = Rc::new(Node {
        name: "root".into(),
        parent: RefCell::new(Weak::new()),
        children: RefCell::new(vec![]),
    });
    let leaf = Rc::new(Node {
        name: "leaf".into(),
        parent: RefCell::new(Weak::new()),
        children: RefCell::new(vec![]),
    });

    *leaf.parent.borrow_mut() = Rc::downgrade(&root);
    root.children.borrow_mut().push(Rc::clone(&leaf));

    println!("leaf's parent = {:?}", leaf.parent.borrow().upgrade().map(|p| p.name.clone()));
    println!("strong={} weak={}", Rc::strong_count(&root), Rc::weak_count(&root));   // 1 1
    println!("children of root: {}", root.children.borrow().len());
}
```

`upgrade()` returns `Option<Rc<T>>`: `None` if the value has already been dropped.

## `Cow<'a, T>`: clone on write

`Cow` ("clone on write") holds either a borrowed or an owned value, cloning only if you need to modify:

```rust
use std::borrow::Cow;

fn normalize(input: &str) -> Cow<str> {
    if input.contains(' ') {
        Cow::Owned(input.replace(' ', "_"))   // allocate only when needed
    } else {
        Cow::Borrowed(input)                  // zero-cost in the common case
    }
}

fn main() {
    println!("{}", normalize("no_spaces"));
    println!("{}", normalize("has some spaces"));
}
```

## Choosing a pointer

```
Need the value on the heap, one owner?                  -> Box<T>
Need many owners, single thread, read-only?             -> Rc<T>
Need many owners AND mutation, single thread?           -> Rc<RefCell<T>>
Need many owners across threads?                        -> Arc<T> (+ Mutex/RwLock if mutated)
Need to avoid cycles / optional back-pointer?           -> Weak<T>
Often borrowed, occasionally owned?                     -> Cow<T>
```

Before you reach for these: could a plain `&T`, a `&mut T`, or an index into a `Vec` do the job? Usually yes.

## Common mistakes

- **Reaching for `Rc<RefCell<T>>` by habit** (from GC languages) when plain ownership works.
- **`RefCell` double borrows** that panic at run time. Keep borrows short; never hold a `borrow_mut()` across a call that might borrow again.
- **Cycles with `Rc`** causing leaks. Use `Weak` for back-references.
- **Using `Rc` across threads** — it won't compile (`Rc` isn't `Send`). Use `Arc`.
- **Calling `.clone()` on an `Rc<T>` expecting a deep copy.** It only increments the count; write `Rc::clone(&x)` to make that obvious.
- **Boxing everything.** A `Box` only helps with recursion, trait objects and big values.

## Exercises

1. Build a binary tree `enum Tree { Leaf, Node(Box<Tree>, i32, Box<Tree>) }` with `insert` and an in-order `to_vec`.
2. Print the strong count of an `Rc<String>` as you create and drop clones.
3. Implement `Drop` for a `Guard(&'static str)` that prints on drop; create three and observe the order.
4. Write a `Counter` with a `Cell<u32>` and a method taking `&self` that increments it.
5. Share a `Vec<String>` between two owners with `Rc<RefCell<Vec<String>>>` and push from each.
6. Add a `parent: Weak<Node>` link to your tree, and print a node's path to the root.
7. Write `fn strip_prefix_cow<'a>(s: &'a str, prefix: &str) -> Cow<'a, str>`.

## Recap

- `Box<T>`: heap allocation with a single owner — for recursive types, trait objects and big moves.
- `Rc<T>` / `Arc<T>`: shared ownership by reference counting (single-threaded / thread-safe).
- `RefCell<T>` / `Cell<T>`: interior mutability — borrow rules checked at run time.
- `Weak<T>` breaks reference cycles; `Cow<T>` avoids clones until needed.
- `Deref` makes smart pointers feel like references; `Drop` runs cleanup automatically.

Next: [Error handling in depth →](08-error-handling.md)
