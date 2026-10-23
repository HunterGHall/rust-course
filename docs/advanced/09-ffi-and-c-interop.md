# 9. FFI & C interoperability

## What & why

Rust rarely lives alone. You'll want to call a C library (SQLite, zlib, OpenSSL, OS APIs), or write a Rust library that C, Python, Swift, Node or Go can call. The bridge is the **FFI** (foreign function interface), and its lingua franca is the **C ABI** — the C calling convention and data layout.

Everything in this lesson is `unsafe` territory: the compiler can't see inside C code. The skill is building a **thin unsafe layer and a safe Rust wrapper** around it.

## Calling C from Rust

Declare the foreign functions in an `unsafe extern` block (edition 2024 requires the `unsafe` keyword on the block) and call them inside `unsafe`:

```rust
use std::ffi::{c_char, c_int, CString};

unsafe extern "C" {
    fn abs(x: c_int) -> c_int;                        // from the C standard library
    fn strlen(s: *const c_char) -> usize;
    safe fn labs(x: i64) -> i64;                      // `safe fn`: declared with no safety requirements
}

fn main() {
    // SAFETY: `abs` has no preconditions.
    let a = unsafe { abs(-5) };

    let text = CString::new("hello").unwrap();       // NUL-terminated, as C expects
    // SAFETY: `text` is a valid NUL-terminated string that outlives the call.
    let n = unsafe { strlen(text.as_ptr()) };

    println!("{a} {n} {}", labs(-9));
}
```

Core pieces:

- **`extern "C"`** — use the C calling convention.
- **`std::ffi::c_int`, `c_char`, `c_long`, `c_void`...** (also `std::os::raw`) — Rust aliases for C's types. Never assume `int` is `i32` on every platform; use the aliases.
- **`CString`** — an owned, NUL-terminated byte string (fails if it contains an interior NUL). **`CStr`** — a borrowed one.
- Linking: the C standard library is linked automatically; for other libraries add `#[link(name = "z")]` on the block, or emit `cargo:rustc-link-lib=z` from a `build.rs`.

### Strings across the boundary

```rust
use std::ffi::{c_char, CStr, CString};

// Pretend this is a C function returning a static, NUL-terminated string.
extern "C" fn c_style_greeting() -> *const c_char {
    c"hello from C land".as_ptr()                    // a C-string literal: &'static CStr
}

fn main() {
    let ptr = c_style_greeting();

    // SAFETY: the pointer is non-null and points to a NUL-terminated string valid for 'static.
    let borrowed: &CStr = unsafe { CStr::from_ptr(ptr) };
    let owned: String = borrowed.to_string_lossy().into_owned();   // copy into a Rust String
    println!("{owned}");

    let to_c = CString::new(owned).expect("no interior NUL bytes");
    println!("{:?}", to_c.as_bytes_with_nul().len());
}
```

Rust's `String`/`&str` are UTF-8 with explicit lengths and **no** NUL terminator; C strings are NUL-terminated and not necessarily UTF-8. Conversion must always happen at the boundary.

## Data layout: `#[repr(C)]`

Rust may reorder struct fields; C may not. For any type shared with C, use `#[repr(C)]`:

```rust
use std::mem::{align_of, size_of};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

#[repr(C)]
struct Header {
    tag: u8,      // 1 byte, then 3 bytes of padding
    length: u32,
    flags: u16,   // + 2 bytes trailing padding
}

#[repr(u8)]       // a fieldless enum with a defined underlying integer type
#[derive(Debug, Clone, Copy)]
enum Status {
    Ok = 0,
    NotFound = 1,
    Denied = 2,
}

fn main() {
    println!("{} {}", size_of::<Point>(), align_of::<Point>());      // 16 8
    println!("{}", size_of::<Header>());                              // 12
    println!("{}", size_of::<Status>());                              // 1
}
```

Other layout attributes: `#[repr(transparent)]` (a wrapper with exactly the layout of its one field — ideal for newtypes), `#[repr(packed)]` (no padding; references to fields may be misaligned and are restricted), and `#[repr(align(N))]`.

Rules of thumb for FFI types:

- Only pass `#[repr(C)]` structs, `repr(u8/i32/...)` fieldless enums, primitives, and pointers.
- Rust `bool`/`char` are *not* guaranteed compatible with C; use `u8`/`u32` (or `c_int`) at the boundary.
- Never pass `String`, `Vec`, `Option<Box<T>>` (except through the documented null-pointer niche), trait objects, or slices (`&[T]` is a fat pointer) directly. Pass a pointer plus a length.
- `Option<extern "C" fn(..)>` and `Option<&T>`/`Option<NonNull<T>>`/`Option<Box<T>>` are ABI-compatible with nullable C pointers.

## Callbacks

Pass a Rust function to C by marking it `extern "C"`:

```rust
use std::ffi::{c_int, c_void};

type Comparator = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

unsafe extern "C" {
    fn qsort(base: *mut c_void, count: usize, size: usize, compare: Comparator);
}

unsafe extern "C" fn compare_i32(a: *const c_void, b: *const c_void) -> c_int {
    // SAFETY: qsort calls us with pointers to elements of the array we passed.
    let (a, b) = unsafe { (*(a as *const i32), *(b as *const i32)) };
    a.cmp(&b) as c_int
}

fn main() {
    let mut numbers = [42, -7, 19, 0, 3];
    // SAFETY: the pointer/length/size describe `numbers` exactly; `compare_i32` matches the element type.
    unsafe {
        qsort(
            numbers.as_mut_ptr() as *mut c_void,
            numbers.len(),
            std::mem::size_of::<i32>(),
            compare_i32,
        );
    }
    println!("{numbers:?}");              // [-7, 0, 3, 19, 42]
}
```

A callback usually needs context. The C convention is a `void *user_data` argument: pass a pointer to your Rust state and cast it back inside the trampoline. **Never let a panic unwind across the boundary** (that is UB for `extern "C"`; since Rust 1.81 such a panic aborts the process). Wrap the body in `std::panic::catch_unwind` if you need to convert a panic into an error code.

## Wrapping a C API safely

The pattern: raw declarations in a private `ffi` module, a safe wrapper type that owns the C resource and frees it in `Drop`:

```rust
use std::ffi::{c_char, c_void, CStr};

mod ffi {
    use super::*;
    unsafe extern "C" {
        pub fn malloc(size: usize) -> *mut c_void;
        pub fn free(ptr: *mut c_void);
        pub fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    }
}

/// A NUL-terminated copy of a string living in C-allocated memory, freed on drop.
/// Callers never see a raw pointer: the unsafe is contained in this type.
pub struct CText {
    ptr: *mut c_char,
    len: usize, // bytes, not counting the NUL
}

impl CText {
    pub fn new(text: &str) -> Option<CText> {
        let len = text.len();
        // SAFETY: malloc has no preconditions; we check for null below.
        let ptr = unsafe { ffi::malloc(len + 1) } as *mut c_char;
        if ptr.is_null() {
            return None;
        }
        // SAFETY: `ptr` has room for len + 1 bytes and `text` has exactly `len` readable bytes;
        // the two regions cannot overlap because `ptr` is a fresh allocation.
        unsafe {
            ffi::memcpy(ptr as *mut c_void, text.as_ptr() as *const c_void, len);
            *ptr.add(len) = 0; // terminating NUL
        }
        Some(CText { ptr, len })
    }

    /// The pointer you would hand to a C function expecting `const char *`.
    pub fn as_ptr(&self) -> *const c_char {
        self.ptr
    }

    pub fn to_str(&self) -> &str {
        // SAFETY: `ptr` is non-null and NUL-terminated, and its contents came from a &str (valid UTF-8).
        let c_str = unsafe { CStr::from_ptr(self.ptr) };
        debug_assert_eq!(c_str.to_bytes().len(), self.len);
        c_str.to_str().expect("constructed from a valid &str")
    }
}

impl Drop for CText {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from malloc and is freed exactly once, here.
        unsafe { ffi::free(self.ptr as *mut c_void) };
    }
}

fn main() {
    let text = CText::new("héllo from malloc").expect("out of memory");
    println!("{}", text.to_str());
    println!("{}", text.as_ptr().is_null());          // false
}   // memory freed here
```

Notice what the wrapper achieves: callers never see a raw pointer, can't forget to free, and can't double-free (`Drop` runs once, and the type is not `Clone`). That's the whole art of FFI in Rust.

## Calling Rust from C

To export a function, make it `extern "C"` and keep its name unmangled. Edition 2024 spells the attribute `#[unsafe(no_mangle)]` because exporting a symbol can break other code:

```rust
use std::ffi::{c_char, c_int, CStr};

/// Adds two numbers. Callable from C as `int rust_add(int, int)`.
#[unsafe(no_mangle)]
pub extern "C" fn rust_add(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

/// Returns the length in characters of a UTF-8 C string, or -1 if invalid.
///
/// # Safety
/// `s` must be a valid pointer to a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_char_count(s: *const c_char) -> c_int {
    if s.is_null() {
        return -1;
    }
    // SAFETY: caller guarantees `s` is a valid C string.
    match unsafe { CStr::from_ptr(s) }.to_str() {
        Ok(text) => text.chars().count() as c_int,
        Err(_) => -1,
    }
}

fn main() {
    println!("{}", rust_add(2, 3));
    println!("{}", unsafe { rust_char_count(c"héllo".as_ptr()) });    // 5
}
```

To build a library C can link against, set the crate type in `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib", "staticlib"]    # shared library (.so/.dll/.dylib) and static archive (.a/.lib)
```

and write a C header that declares the functions (by hand, or generate it with **`cbindgen`**):

```c
// mylib.h
int rust_add(int a, int b);
int rust_char_count(const char *s);
```

Returning heap data to C means giving C a way to free it. Export a matching `free` function from Rust — **memory must be freed by the same allocator that allocated it**:

```rust
use std::ffi::{c_char, CString};

#[unsafe(no_mangle)]
pub extern "C" fn rust_make_greeting(name: *const c_char) -> *mut c_char {
    let name = if name.is_null() {
        "world".to_string()
    } else {
        // SAFETY: documented requirement — `name` is a valid NUL-terminated string.
        unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().into_owned()
    };
    CString::new(format!("Hello, {name}!")).unwrap().into_raw()   // ownership passes to C
}

/// # Safety
/// `s` must come from `rust_make_greeting` and not have been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_free_string(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: reconstitute the CString we released with into_raw, then drop it.
        drop(unsafe { CString::from_raw(s) });
    }
}

fn main() {
    let p = rust_make_greeting(std::ptr::null());
    println!("{}", unsafe { std::ffi::CStr::from_ptr(p) }.to_str().unwrap());
    unsafe { rust_free_string(p) };
}
```

## Opaque handles

To hide a Rust object from C, hand out a pointer to a boxed value and provide `new` / methods / `free`:

```rust
pub struct Counter {
    count: u64,
}

#[unsafe(no_mangle)]
pub extern "C" fn counter_new() -> *mut Counter {
    Box::into_raw(Box::new(Counter { count: 0 }))
}

/// # Safety
/// `c` must be a live pointer from `counter_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn counter_increment(c: *mut Counter) -> u64 {
    // SAFETY: caller guarantees `c` is live and uniquely accessed.
    let counter = unsafe { &mut *c };
    counter.count += 1;
    counter.count
}

/// # Safety
/// `c` must come from `counter_new` and not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn counter_free(c: *mut Counter) {
    if !c.is_null() {
        // SAFETY: reclaims the Box created in counter_new.
        drop(unsafe { Box::from_raw(c) });
    }
}

fn main() {
    let c = counter_new();
    unsafe {
        counter_increment(c);
        println!("{}", counter_increment(c));    // 2
        counter_free(c);
    }
}
```

On the C side `Counter` is just `typedef struct Counter Counter;` — an incomplete type that can only be used through pointers.

## Generating bindings and building C code

You rarely write big `extern` blocks by hand:

- **`bindgen`** reads C headers and generates the Rust declarations (types, functions, constants). Typically run from `build.rs`, or via the `-sys` crate convention: a crate named `foo-sys` contains only raw bindings (and builds or locates the C library), and a crate `foo` provides the safe wrapper.
- **`cbindgen`** goes the other way: it reads your Rust `extern "C"` API and writes the C/C++ header.
- **`cc`** compiles C/C++ sources from `build.rs`:

<!-- check: skip -->
```rust
// build.rs  (add `cc = "1"` under [build-dependencies])
fn main() {
    cc::Build::new()
        .file("src/helper.c")
        .compile("helper");                     // produces libhelper.a and links it
    println!("cargo:rerun-if-changed=src/helper.c");
}
```

For C++ there's the `cxx` crate (safe, bidirectional Rust ↔ C++ with generated glue), and for Python, Node, Java, Swift there are `PyO3`, `napi-rs`, `jni`, and `uniffi`, all built on this C-ABI foundation.

## Strategy checklist for an FFI boundary

1. Define the narrowest possible C interface (opaque handles, plain integers, pointer + length).
2. Document ownership of every pointer: who allocates, who frees, how long it stays valid.
3. Validate everything coming in (null pointers, lengths, UTF-8); never trust the other side.
4. Never unwind across the boundary; use `catch_unwind` or `panic = "abort"`.
5. Free memory with the allocator that created it (pair every `*_new` with a `*_free`).
6. Test the C side too (a small C program, ASan, Valgrind) and run Miri on the Rust side where possible.

## Common mistakes

- **Using Rust types in the signature** (`&str`, `String`, `Vec<T>`, tuples, trait objects) of an `extern "C"` function.
- **Forgetting `#[repr(C)]`** on shared structs.
- **Freeing Rust-allocated memory with C's `free`** (or the reverse) — undefined behaviour.
- **Handing C a pointer to a temporary:** `CString::new("x").unwrap().as_ptr()` dangles at the end of the statement; keep the `CString` alive in a variable.
- **Assuming `char`/`int`/`long` sizes** — use `c_char`, `c_int`, `c_long`.
- **Ignoring return codes and `errno`.**
- **Letting a panic cross the boundary.**
- **Linking errors** from forgetting `#[link]`/`build.rs` output or mixing debug and release C libs.

## Exercises

1. Call `getenv` from the C library through FFI to read `HOME` (or `USERPROFILE`) and print it as a Rust `String`, handling the null (unset) case.
2. Call `qsort` on an array of `f64` using a callback that handles NaN deterministically.
3. Write a safe wrapper `fn c_strlen(s: &str) -> Result<usize, NulError>` around `strlen`.
4. Create a `cdylib` crate exporting `rust_add` and `rust_char_count`, write a 10-line C program that calls them, and compile and run it.
5. Add `counter_get` and a `counter_reset` function to the opaque `Counter`, and call everything from C.
6. Use `cbindgen` to generate the header for exercise 4 and compare it with the one in this lesson.
7. Make a callback with a `user_data` pointer that collects results into a `Vec<i32>` owned by Rust.

## Recap

- FFI uses the C ABI: `extern "C"`, `#[repr(C)]`, C types from `std::ffi`, `CString`/`CStr` for text.
- Declare foreign functions in an `unsafe extern` block; export with `#[unsafe(no_mangle)] pub extern "C" fn`.
- Wrap raw pointers in safe types that own resources and free them in `Drop`.
- Memory is freed by whoever allocated it; never unwind across the boundary.
- `bindgen`, `cbindgen`, `cc`, `cxx`, `PyO3` and friends automate the plumbing.

Next: [Performance & profiling →](10-performance.md)
