Thinking: Perfect! I've confirmed the bug. Here's what happened:
1. In Darcy DSL: (defrecord pixel (x i32) (y i32) (r u8) (g u8) (b u8))
2. Darcy compiles this to Rust as: pub r: U8, pub g: U8, pub b: U8,
3. Rust rejects this because U8 doesn't exist - only u8 (lowercase) exists
This is a clear bug in the Darcy compiler's type lowering - it's capitalizing primitive types when it shouldn't.
Let me show the exact Darcy source that triggers the bug.
Exact Code Demonstrating the u8 Type Bug
Darcy Source Code:
(defrecord pixel (x i32) (y i32) (r u8) (g u8) (b u8))
(defn test []
  (pixel 0 0 255 128 64))
Generated Rust Code (from cargo run -p dslc):
#[derive(Debug, Clone)]
pub struct Pixel {
    pub x: i32,
    pub y: i32,
    pub r: U8,    // ❌ BUG: Should be `u8`
    pub g: U8,    // ❌ BUG: Should be `u8`
    pub b: U8,    // ❌ BUG: Should be `u8`
}
fn test() -> Pixel {
    Pixel { x: 0i32, y: 0i32, r: 255u8, g: 128u8, b: 64u8 }
}
The Problem:
Darcy compiler incorrectly lowers u8 to U8 (capital U) instead of u8 (lowercase u).
Rust Compilation Error:
error[E0412]: cannot find type `U8` in this scope
  --> test.rs:8:12
   |
8  |     pub r: U8,
   |            ^^ help: a builtin type with a similar name exists (notice the capitalization): `u8`
Why This Forced Workarounds:
In the fractal generator, I had to avoid u8 entirely:
; Can't do this (bug):
(defrecord pixel (px i32) (py i32) (pr u8) (pg u8) (pb u8))
; Had to do this instead (workaround):
(defrecord pixel (px i32) (py i32) (pr i32) (pg i32) (pb i32))
Then cast to u8 only when saving the image:
// In Rust main.rs:
let color = Rgb([pixel.pr as u8, pixel.pg as u8, pixel.pb as u8]);
Summary:
- Darcy source: Uses correct type name u8
- Darcy compiler: Incorrectly emits U8 in Rust code
- Rust: Rejects U8 because only lowercase u8 is a valid primitive type
- Impact: Cannot use u8 in Darcy records, forcing i32 + casting workarounds
▣  Build · glm-4.7 · 1m 48s