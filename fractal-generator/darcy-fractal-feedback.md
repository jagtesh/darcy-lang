# Darcy Fractal Generator - Development Challenges & Feedback

This document outlines the challenges and problems encountered while developing a fractal generator in Darcy, along with suggestions for improvement.

## Major Challenges and Problems

### 1. Syntax and Naming Constraints

- **Reserved keywords everywhere**: `type`, `vec`, `not`, `mod` are all reserved, forcing awkward workarounds
- **Field naming rules**: Had to rename pixel fields from `x,y,r,g,b` to `px,py,pr,pg,pb` to avoid conflicts
- **Module prefix confusion**: `darcy.vec` can't be aliased as `vec` (reserved), leading to verbose `darcy.vec/push` everywhere

### 2. Type System Frustrations

- **`u8` type broken**: Darcy compiles `u8` to `U8` (wrong capitalization), causing Rust compilation errors
- **Type mismatches**: Mixing `u8` for colors with `i32` for arithmetic required constant casting
- **Had to abandon u8 entirely**: Used `i32` for everything and cast at the very end when saving to image

**Status:** RESOLVED. Primitive integer types now map directly to Rust (`u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128`).

Example of the problem:
```darcy
; In generated Rust:
pub pr: U8,  // Wrong! Should be u8
pub pg: U8,
pub pb: U8,
```

### 3. Missing Core Operators

No built-in modulo for integers! Had to implement this manually:

```darcy
(defin mod-custom [x:i64 m:i64]
  (- x (* m (cast (/ (cast x f64) (cast m f64)) i64))))
```

This is fundamental - modulo should be built-in as a core math operator.

**Status:** RESOLVED. `mod` is now a built-in integer operator.

### 4. Confusing Loop Semantics

- `loop` form with explicit `continue`/`break` didn't work as expected
- Had to switch to `while` which is simpler but more verbose
- The documentation shows `loop (while cond expr)` but this syntax was unclear and error-prone

Attempted pattern that failed:
```darcy
(loop i 0
  (if (math/gt i max)
    max
    (do
      (let! z ...)
      (let! i (+ i 1))
      (continue))))  ; This caused syntax errors
```

### 5. Function Visibility is Counterintuitive

```darcy
; This won't be visible to Rust:
(defn generate-mandelbrot [width:i32 height:i32 max-iter] ...)

; Must explicitly wrap with export:
(export (defn generate-mandelbrot [width:i32 height:i32 max-iter] ...))
```

This should be automatic or have a clearer marker like `pub` syntax similar to Rust.

**Status:** PARTIAL. `main` is auto-exported, but other functions still require `export`.

### 6. Vector Mutability is Awkward

Expected this to work:
```darcy
(let! pixels (darcy.vec/new))
(while ...
  (darcy.vec/push pixels value))
```

Actually needed this:
```darcy
(let! pixels (darcy.vec/new))
(while ...
  (let! pixels (darcy.vec/push (darcy.core/clone pixels) value)))
```

**Problem**: Vector operations return new vectors rather than mutating, forcing explicit cloning in loops. Rust's `Vec<T>` has push returning `()` but Darcy's abstraction forces copying the entire vector each time.

**Status:** IMPROVED. Vectors are now lowered to `Arc<Vec<T>>` and variable access auto-clones the `Arc`. You still rebind with `let!`, but no explicit `core/clone` is needed, and the runtime uses `Arc::make_mut` to avoid copying when uniquely owned.

### 7. Error Messages and Diagnostics

- **Errors point to wrong code**: "auto-cloned 'pixels' to avoid move" warning points to generated Rust code, not Darcy source
- **Limited suggestions**: Errors like "unknown module 'darcy.vec'" don't suggest how to import it
- **Type mismatch details**: Could be more helpful about what types were actually inferred vs expected

Example of confusing error:
```
warning: auto-cloned 'pixels' to avoid move; consider darcy.core/clone
warning: fractal-generator@0.1.0:  --> /Volumes/Dev/.../darcy/main.dsl:102:5
```
This points to the generated Rust file location, not the original Darcy source.

### 8. let Binding Syntax Confusion

```darcy
; This syntax is valid but unclear:
(let [x 1 y 2 z (+ x y)] ...)

; Flat pairs vs nested - both allowed:
(let [(x 1) (y 2)] ...)  
(let [x 1 y 2] ...)
```

Both are allowed but the differences and when to use which aren't documented well. This creates confusion about scoping and evaluation order.

## What Should Have Been Simpler

### Immediate Fixes Needed

1. **Fix `u8` type capitalization** - This is a compiler bug that blocks basic types
2. **Add built-in `mod` operator** - Should be core math, not require manual implementation
3. **Better default visibility** - Make functions exportable without explicit `(export (defn ...))` wrapper
4. **Fix vector push mutation** - Don't force cloning in loops; support mutation or make immutability requirements clearer

### Language Design Improvements

1. **Fewer reserved words**: Only truly essential keywords should be reserved
2. **Better function markers**: `pub defn` instead of `(export (defn ...))`
3. **Clearer mutability**: Either support mutation directly or make immutability more explicit
4. **Improved error messages**: Source-level error tracing with suggested fixes
5. **Standard operators**: All basic math operators should be built-in (modulo, bitwise, etc.)
6. **Better loop constructs**: Simplified syntax that doesn't require complex nesting or confusing keywords

### Documentation Gaps

1. **Concrete examples**: LANGUAGE.md needs more complete working examples, not just snippets
2. **Common patterns**: How to write loops, handle mutability, work with vectors effectively
3. **Rust interop guide**: Clearer explanation of `export` and type mapping
4. **Error troubleshooting**: Common errors and how to fix them
5. **Performance considerations**: When cloning happens, how to avoid it, memory implications

## Specific Issues with This Project

### Complex Number Arithmetic

Working with complex numbers required implementing all operations as inline functions:
```darcy
(defin complex-abs-sq [c:complex]
  (+ (* c.re c.re) (* c.im c.im)))

(defin complex-add [c1:complex c2:complex]
  (complex (+ c1.re c2.re) (+ c1.im c2.im)))

(defin complex-mul [c1:complex c2:complex]
  (complex (- (* c1.re c2.re) (* c1.im c2.im)) 
          (+ (* c1.re c2.im) (* c1.im c2.re))))
```

While this works, it would be better to have a standard library module for complex numbers or built-in operators.

### Nested Binding Confusion

The code ended up with deeply nested `let` expressions to avoid syntax errors:
```darcy
(defn generate-fractal [config:fractal-config]
  (let [pixels (darcy.vec/new)]
    (let [y 0]
      (while (math/lt y config.height)
        (do
          (let [x 0]
            (while (math/lt x config.width)
              (do
                (let! pixels (darcy.vec/push (darcy.core/clone pixels) (generate-pixel config x y)))
                (let! x (+ x 1)))))
          (let! y (+ y 1)))))
    pixels))
```

This could be much cleaner with better scoping or block expressions.

## Core Assessment

The fundamental issue is that Darcy tries to be a "clean Lisp" but adds enough Rust-specific complexities that simplicity is lost, without providing enough of Rust's tooling to make the complexities manageable.

### The Tradeoff Problem

Darcy aims to:
- Provide Lisp-like syntax
- Compile to safe, fast Rust
- Have clear, predictable types

But it achieves this by:
- Adding many reserved words that clash with common identifiers
- Creating a leaky abstraction around Rust's ownership model
- Requiring explicit patterns for operations that are simple in other languages

### What Works Well

- **Type inference** for primitives is generally good
- **Pattern matching** with `case` is clean and powerful
- **Record/Enum syntax** is readable and intuitive
- **Module system** with `require` works well once you understand the rules

### What Needs Work

- **Error messages and debugging**: Developers spend too much time understanding compiler errors
- **Standard library**: Missing basic utilities (modulo, complex numbers, etc.)
- **Mutability model**: The current approach forces inefficient patterns
- **Documentation**: Needs more real-world examples and troubleshooting guides

## Recommendations

### For Language Design

1. Adopt `pub defn` syntax instead of `(export (defn ...))`
2. Make `mod` and basic math operators built-in
3. Fix the `u8` type capitalization bug
4. Consider supporting mutation in certain contexts (like vector push in loops)
5. Reduce the number of reserved words
6. Improve error messages to point to source locations and suggest fixes

### For Documentation

1. Add a "Cookbook" section with complete, working examples
2. Document common patterns (loops, vectors, mutations)
3. Create a troubleshooting guide for common errors
4. Add performance considerations and optimization tips
5. Provide a Rust interop deep-dive guide

### For Tooling

1. Better editor integration with real-time type checking
2. Improved error reporting with source location tracking
3. LSP support with code suggestions
4. Better build error messages that reference Darcy source, not generated Rust

## Conclusion

Developing a complex project in Darcy reveals that while the language has good foundations, the current implementation has several rough edges that make development frustrating. The core concepts are sound, but the practical experience is hindered by:

- Too many reserved keywords
- Missing standard operators
- Confusing mutability semantics
- Poor error messages
- Incomplete documentation

With focused improvements in these areas, Darcy could be a much more pleasant and productive language for this type of systems programming.
