# Darcy Compiler - Issue Status Check

## ✅ Fixed Issues

### 1. u8 Type Capitalization Bug - **FIXED**

**Before (Old Version):**
```rust
pub struct Pixel {
    pub r: U8,    // ❌ Wrong - should be u8
    pub g: U8,    // ❌ Wrong - should be u8
    pub b: U8,    // ❌ Wrong - should be u8
}
```

**After (Current Version):**
```rust
pub struct Pixel {
    pub r: u8,    // ✅ Correct!
    pub g: u8,    // ✅ Correct!
    pub b: u8,    // ✅ Correct!
}
```

**Test:**
```darcy
(defrecord pixel (x i32) (y i32) (r u8) (g u8) (b u8))

(defn test []
  (pixel 0 0 255 128 64))
```

Compiles to valid Rust and compiles without errors!

---

## ❌ Still Problematic Issues

### 1. Reserved Keywords Remain

**Test - `type` is still reserved:**
```darcy
(defrecord test-type (type i32))  ; Error: 'type' is a reserved keyword
```

**Test - `vec` cannot be aliased:**
```darcy
(require [darcy.vec :as vec])  ; Error: 'vec' is a reserved keyword
```

**Impact:** Forces awkward naming like `fractal-type`, `test-type`, etc.

---

### 2. Missing Modulo Operator

**Status:** No built-in `mod` operator.

**Workaround still required:**
```darcy
(defin mod-custom [x:i64 m:i64]
  (- x (* m (cast (/ (cast x f64) (cast m f64)) i64))))
```

**Expected:**
```darcy
; Should work but doesn't:
(defn test [x:i64 m:i64]
  (mod x m))
```

---

### 3. Verbose Export Syntax

**Status:** Export mechanism works but is verbose.

**Current syntax:**
```darcy
(defn internal-func [x:i32]
  x)

(export (defn exported-func [x:i32]
  x))
```

**Generated Rust:**
```rust
fn internal_func(x: i32) -> i32 {
    x
}

pub fn exported_func(x: i32) -> i32 {
    x
}
```

**Desired syntax (doesn't exist):**
```darcy
; Would be cleaner:
(pub defn exported-func [x:i32]
  x)
```

---

### 4. No Built-in Complex Numbers

**Status:** Complex numbers require manual implementation.

**Current workaround:**
```darcy
(defrecord complex (re f64) (im f64))

(defin complex-add [c1:complex c2:complex]
  (complex (+ c1.re c2.re) (+ c1.im c2.im)))

(defin complex-mul [c1:complex c2:complex]
  (complex (- (* c1.re c2.re) (* c1.im c2.im)) 
          (+ (* c1.re c2.im) (* c1.im c2.re))))
```

**Expected:** Built-in complex type and operations in stdlib.

---

## Summary

### What Was Fixed:
- ✅ `u8` type capitalization - now correctly generates `u8` instead of `U8`
- ✅ Automatic cloning - compiler handles it without manual `darcy.core/clone` calls
- ✅ Loop patterns - `while` loops work cleanly with `let!`
- ✅ Helpful warnings - compiler suggests fixes proactively

### What Still Needs Work:
- ❌ Reserved keywords (`type`, `vec`, etc.) force awkward naming
- ❌ Missing basic operators (`mod`, bitwise, etc.)
- ❌ Verbose export syntax requires `(export (defn ...))` wrapper
- ❌ No built-in complex numbers
- ❌ Vector mutation performance - automatic cloning still has overhead
- ❌ Type annotation syntax issues - `[x:i32]` vs just `[x]`

### Core Assessment:
The language has improved significantly - the `u8` bug fix alone makes it much more practical. However, fundamental design issues remain:

1. Too many reserved words restricts naming
2. Missing stdlib operations require manual implementation
3. Verbose syntax patterns could be more concise
4. Performance concerns (vector cloning) not addressed

Darcy is now **much more usable** than before, but still has rough edges that could be smoothed out for better developer experience.
