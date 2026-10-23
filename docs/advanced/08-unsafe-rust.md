# 8. Unsafe Rust

## What & why

Everything so far has been **safe Rust**, where the compiler guarantees memory safety. But the compiler is conservative: some correct programs can't be proven safe by its rules, and some things (talking to hardware, calling C, implementing `Vec` itself) are inherently outside what it can check. For those, Rust has an escape hatch: the **`unsafe`** keyword.

`unsafe` does **not** turn off the borrow checker or other checks. It unlocks exactly five extra abilities and says: *"the compiler can't verify this part; I, the programmer, promise it upholds Rust's rules."*

## The five unsafe superpowers

Inside an `unsafe` block (or `unsafe fn`) you may:

1. Dereference a **raw pointer** (`*const T`, `*mut T`).
2. Call an **`unsafe` function** or method (including foreign functions).
3. Access or modify a **mutable `static`**.
4. Implement an **`unsafe` trait** (`Send`, `Sync`, ...).
5. Access fields of a **`union`**.

Everything else — type checking, borrowing of references, ownership — still applies.

### The contract

Undefined behaviour (UB) is the thing to fear. It's not "a crash" — it's "the compiler is allowed to assume it never happens, so the program may do *anything*, including appearing to work". UB includes:

- dereferencing a null, dangling or misaligned pointer,
- data races,
- breaking reference rules (an `&mut` aliasing another reference, or an `&T` whose pointee is mutated),
- producing an invalid value (a `bool` that isn't 0/1, an enum with a bad discriminant, a dangling `&T`, uninitialized integers),
- reading uninitialized memory,
- unwinding across an FFI boundary where it isn't allowed,
- violating a function's documented safety requirements.

The goal of unsafe code is to encapsulate this risk in a small, well-reviewed area behind a **safe API** that cannot be misused.

## Raw pointers

```rust
fn main() {
    let mut value = 10;

    let shared: *const i32 = &value;          // creating raw pointers is safe...
    unsafe {                                   // ...dereferencing them is not
        println!("{}", *shared);               // 10
    }

    let exclusive: *mut i32 = &mut value;
    unsafe {
        *exclusive += 5;
        println!("{}", *exclusive);            // 15
        // Reading through `shared` here would be undefined behaviour: creating `exclusive`
        // (from `&mut value`) invalidated it. Miri reports exactly this.
    }

    let null: *const i32 = std::ptr::null();
    println!("{}", null.is_null());            // true — check before dereferencing!
}
```

Raw pointers differ from references: they can be null, dangling or aliased, and the borrow checker doesn't track them. But that does **not** mean anything goes — the aliasing rules still apply to the *data* you access, and breaking them is UB even though the compiler stays quiet. (This is the kind of mistake the Miri tool, introduced below, catches.) Pointer arithmetic:

```rust
fn main() {
    let numbers = [10, 20, 30, 40];
    let base: *const i32 = numbers.as_ptr();

    unsafe {
        println!("{}", *base.add(2));                    // 30 — add counts in elements, not bytes
        println!("{}", base.add(3).read());               // 40
        let past_end = base.add(4);                       // allowed: one past the end (never dereference it)
        println!("{}", past_end.offset_from(base));       // 4
    }
}
```

Calling `add` is itself UB if the result leaves the original allocation (more than one past the end) — which is why it is `unsafe`.

## `unsafe fn` and documenting safety

An `unsafe fn` is one whose correctness depends on conditions the *caller* must guarantee. Document them in a `# Safety` section, and justify every `unsafe` block with a `// SAFETY:` comment:

```rust
/// Returns the element at `index` without bounds checking.
///
/// # Safety
///
/// `index` must be less than `slice.len()`.
unsafe fn get_unchecked_demo(slice: &[i32], index: usize) -> i32 {
    // SAFETY: the caller guarantees `index < slice.len()`.
    unsafe { *slice.as_ptr().add(index) }
}

fn main() {
    let data = [1, 2, 3];
    let i = 1;
    assert!(i < data.len());
    // SAFETY: we just checked that `i` is in bounds.
    let value = unsafe { get_unchecked_demo(&data, i) };
    println!("{value}");
}
```

Edition 2024 warns (`unsafe_op_in_unsafe_fn`) if you perform unsafe operations in an `unsafe fn` without an inner `unsafe { }` block, so each unsafe operation is explicit and individually justified.

## Safe abstractions over unsafe code

The standard pattern: write a small unsafe core, then wrap it in a safe API whose types make misuse impossible. A classic example is splitting one `&mut [T]` into two non-overlapping halves — safe Rust can't express it, but it is obviously sound:

```rust
use std::slice;

fn split_at_mut_demo<T>(values: &mut [T], mid: usize) -> (&mut [T], &mut [T]) {
    let len = values.len();
    let ptr = values.as_mut_ptr();
    assert!(mid <= len);                       // the check that makes the unsafe block sound

    // SAFETY: both ranges lie within `values`, and they don't overlap:
    // [0, mid) and [mid, len). The input &mut borrow is consumed, so no other alias exists.
    unsafe {
        (
            slice::from_raw_parts_mut(ptr, mid),
            slice::from_raw_parts_mut(ptr.add(mid), len - mid),
        )
    }
}

fn main() {
    let mut numbers = [1, 2, 3, 4, 5];
    let (left, right) = split_at_mut_demo(&mut numbers, 2);
    left[0] = 100;
    right[0] = 300;
    println!("{numbers:?}");               // [100, 2, 300, 4, 5]
}
```

A function is **sound** if no sequence of *safe* calls can trigger UB; if one can, it is **unsound** — a bug in the library even though no one wrote `unsafe` at the call site. Notice `assert!(mid <= len)`: without that check, safe callers could cause UB.

## A worked example: a tiny `Vec`

Here is a minimal growable array built from scratch with the allocator API. It shows ownership of raw memory, `Drop`, and why unsafe code needs care:

```rust
use std::alloc::{self, Layout};
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};

pub struct MiniVec<T> {
    ptr: NonNull<T>,    // pointer to the start of the buffer (dangling while cap == 0)
    cap: usize,         // how many elements fit
    len: usize,         // how many are initialized
}

impl<T> MiniVec<T> {
    pub fn new() -> MiniVec<T> {
        assert!(std::mem::size_of::<T>() != 0, "zero-sized types are not supported here");
        MiniVec { ptr: NonNull::dangling(), cap: 0, len: 0 }
    }

    fn grow(&mut self) {
        let new_cap = if self.cap == 0 { 4 } else { self.cap * 2 };
        let new_layout = Layout::array::<T>(new_cap).expect("capacity overflow");

        let raw = if self.cap == 0 {
            // SAFETY: new_layout has non-zero size (T is not zero-sized and new_cap >= 4).
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            // SAFETY: self.ptr was allocated with old_layout; new size is non-zero.
            unsafe { alloc::realloc(self.ptr.as_ptr() as *mut u8, old_layout, new_layout.size()) }
        };

        self.ptr = match NonNull::new(raw as *mut T) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }

    pub fn push(&mut self, value: T) {
        if self.len == self.cap {
            self.grow();
        }
        // SAFETY: len < cap, so this slot is inside the allocation and currently uninitialized;
        // `write` doesn't drop the (nonexistent) old value.
        unsafe { ptr::write(self.ptr.as_ptr().add(self.len), value) };
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            // SAFETY: the slot at the old last index was initialized; after decrementing
            // `len` we treat it as uninitialized, so ownership moves out exactly once.
            unsafe { Some(ptr::read(self.ptr.as_ptr().add(self.len))) }
        }
    }
}

impl<T> Drop for MiniVec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            while self.pop().is_some() {}           // drop each remaining element
            let layout = Layout::array::<T>(self.cap).unwrap();
            // SAFETY: allocated by `grow` with exactly this layout, and freed only here.
            unsafe { alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout) };
        }
    }
}

impl<T> Deref for MiniVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        // SAFETY: the first `len` elements are initialized; the pointer is non-null and aligned.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for MiniVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: as above, and `&mut self` guarantees exclusive access.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

// SAFETY: MiniVec owns its elements like Vec<T> does, so it is as thread-safe as T.
unsafe impl<T: Send> Send for MiniVec<T> {}
unsafe impl<T: Sync> Sync for MiniVec<T> {}

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut v = MiniVec::new();
    for n in 1..=10 {
        v.push(n * n);
    }
    println!("{} {:?}", v.len(), &v[..3]);        // slice methods come from Deref
    println!("{:?} {:?}", v.pop(), v.iter().max());
    v.sort_by(|a, b| b.cmp(a));                    // DerefMut gives us sort_by for free
    println!("{:?}", &v[..3]);

    let mut names = MiniVec::new();
    names.push(Noisy("a"));
    names.push(Noisy("b"));
    names.push(Noisy("c"));
    let popped = names.pop();                      // moves "c" out
    println!("popped {}", popped.as_ref().unwrap().0);
    drop(names);                                   // drops "a" and "b" exactly once
    println!("end of main");                       // then `popped` drops "c"
}
```

What the real `Vec` adds: zero-sized-type handling, `insert`/`remove`, iterators, `into_iter`, panic-safety during `clone`/`extend`, `Layout` overflow checks, and thousands of tests plus Miri runs. The lesson: even a toy needs a careful written justification for every `unsafe` block.

## Unsafe traits

A trait is `unsafe` when implementing it carries obligations the compiler can't check. `Send` and `Sync` are the main ones:

```rust
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

/// A tiny spin lock guarding a value.
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,             // UnsafeCell is the ONLY legal way to mutate through a shared reference
}

// SAFETY: access to `value` is serialized by the `locked` flag, so sharing a
// `SpinLock<T>` between threads is safe whenever `T` can be sent between them.
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        SpinLock { locked: AtomicBool::new(false), value: UnsafeCell::new(value) }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        // SAFETY: we hold the lock, so no other thread has access to the value.
        let result = f(unsafe { &mut *self.value.get() });
        self.locked.store(false, Ordering::Release);
        result
    }
}

static COUNTER: SpinLock<u32> = SpinLock::new(0);

fn main() {
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                for _ in 0..1000 {
                    COUNTER.with(|n| *n += 1);
                }
            });
        }
    });
    println!("{}", COUNTER.with(|n| *n));       // 4000
}
```

(This isn't panic-safe: a panic inside `f` would leave the lock held. Production code uses a guard type with `Drop`. Prefer `std::sync::Mutex` — this is purely illustrative.)

## Mutable statics

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

static mut LEGACY_COUNTER: u32 = 0;
static REQUESTS: AtomicUsize = AtomicUsize::new(0);

fn main() {
    // A `static mut` is a global with no synchronization: every access is unsafe, and
    // creating a reference to it (`&LEGACY_COUNTER`) is denied by default in edition 2024.
    unsafe {
        LEGACY_COUNTER += 1;
        let value = LEGACY_COUNTER;               // copy the value out; never take a reference
        println!("{value}");
    }

    // Almost always better: a safe global.
    REQUESTS.fetch_add(1, Ordering::Relaxed);
    println!("{}", REQUESTS.load(Ordering::Relaxed));
}
```

Prefer atomics, `Mutex`, `OnceLock`, or `LazyLock` for global state.

## Uninitialized memory and `MaybeUninit`

You may never create a value of a type with uninitialized contents (`let x: u32 = mem::uninitialized()` is UB, and removed). To build arrays element by element, use `MaybeUninit`:

```rust
use std::mem::MaybeUninit;

fn squares() -> [u32; 5] {
    let mut array: [MaybeUninit<u32>; 5] = [const { MaybeUninit::uninit() }; 5];
    for (i, slot) in array.iter_mut().enumerate() {
        slot.write((i * i) as u32);                // initialize each slot
    }
    // SAFETY: all five elements were initialized above; MaybeUninit<u32> and u32 have the same layout.
    unsafe { std::mem::transmute::<[MaybeUninit<u32>; 5], [u32; 5]>(array) }
}

fn main() {
    println!("{:?}", squares());                    // [0, 1, 4, 9, 16]
}
```

(In real code, `std::array::from_fn(|i| (i * i) as u32)` does this safely.)

## `transmute` and `union`

`mem::transmute` reinterprets the bits of one type as another of the same size. It's powerful and dangerous; there's almost always a safer alternative:

```rust
fn main() {
    let bits: u32 = unsafe { std::mem::transmute(1.0f32) };   // works...
    let better = 1.0f32.to_bits();                              // ...but this is the safe API
    println!("{bits:#x} {better:#x}");                          // 0x3f800000

    let bytes = 0x12345678u32.to_le_bytes();                    // instead of transmuting to [u8; 4]
    println!("{bytes:x?}");

    let c = char::from_u32(0x41);                               // instead of transmuting u32 -> char
    println!("{c:?}");                                          // not every u32 is a valid char!
}
```

A `union` stores several fields in the same memory; reading one is unsafe because the compiler can't know which was last written. They're mainly for FFI.

## Tools for verifying unsafe code

| Tool | What it finds |
| --- | --- |
| **Miri** (`cargo +nightly miri test`) | an interpreter that detects UB: out-of-bounds, use-after-free, invalid values, aliasing violations, data races, leaks |
| **Sanitizers** (`RUSTFLAGS=-Zsanitizer=address`, `thread`, `memory`) | memory errors and data races at run time |
| **Loom** | exhaustively explores thread interleavings for concurrent data structures |
| **cargo-geiger** | counts `unsafe` in your dependency tree |
| **Clippy** (`undocumented_unsafe_blocks`) | flags `unsafe` without a `// SAFETY:` comment |
| **Kani / Prusti / Verus** | formal verification of Rust code |

Run Miri on every crate that contains `unsafe`:

```bash
rustup +nightly component add miri
cargo +nightly miri test
```

## Guidelines

- **Don't use unsafe unless you must** — to call FFI, to write a primitive data structure, or after profiling shows a real need. "It's faster" is almost never true without measurements, and the safe version is often the same speed.
- **Keep unsafe blocks tiny** and put the safe API boundary right around them.
- **Write the `// SAFETY:` comment first**; if you can't write it, you don't understand why the code is sound.
- **Make invariants private** (fields hidden, constructors checked) so safe code can't break them.
- **Test with Miri** and keep `#![deny(unsafe_op_in_unsafe_fn)]` on.
- **Treat "it works on my machine" as no evidence at all.**

## Common mistakes

- **Dereferencing a pointer after the owner is gone or moved** (dangling).
- **Creating two `&mut` to the same data** through raw pointers — instant UB even if you never use both at once.
- **Forgetting that `Vec::set_len` or `ptr::write`/`read` skip drops**, causing double-drops or leaks.
- **Reading uninitialized memory**, including via `mem::zeroed()` for types where zero is invalid (references, `NonZero`, `Box`).
- **Assuming a layout** without `#[repr(C)]` — Rust may reorder struct fields.
- **Unwinding past FFI**; mark callbacks `extern "C"` and catch panics.
- **Implementing `Send`/`Sync` by hand** without proving the data can really cross threads.

## Exercises

1. Write `fn swap_raw<T>(a: *mut T, b: *mut T)` using `ptr::swap`, document its `# Safety` section, and call it correctly.
2. Reimplement `slice::split_at_mut` for `&mut [i32]` and show it rejects `mid > len`.
3. Add `insert(&mut self, index: usize, value: T)` to `MiniVec` using `ptr::copy`, with bounds checking.
4. Add an `IntoIterator` for `MiniVec` that yields owned elements; make sure remaining elements drop when the iterator is dropped early.
5. Write a `RawBuffer` struct that owns a `*mut u8` from `alloc::alloc` and frees it in `Drop`; try double-freeing by cloning it with `#[derive(Clone)]` and see why that's wrong.
6. Replace each `transmute` in your code with a safe API (`to_bits`, `from_ne_bytes`, `char::from_u32`).
7. Install Miri and run it on `MiniVec`; then deliberately remove the `assert!(mid <= len)` from `split_at_mut_demo` and see Miri report the error.

## Recap

- `unsafe` unlocks five abilities (raw pointer deref, unsafe calls, mutable statics, unsafe traits, unions); it doesn't disable other checks.
- UB means anything may happen; unsafe code must uphold Rust's rules manually.
- Build a **safe API** around a **small unsafe core**, and justify every block with `// SAFETY:`.
- A safe function that can cause UB is *unsound* — a bug even though callers wrote no `unsafe`.
- Verify with Miri, sanitizers and tests; prefer safe alternatives (`to_bits`, atomics, `from_fn`) whenever they exist.

Next: [FFI & C interoperability →](09-ffi-and-c-interop.md)
