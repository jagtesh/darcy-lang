# Challenges Coding in Darcy (Updated 2)

This document outlines the friction points and challenges encountered while implementing a simple trading simulation in Darcy.

## 1. Strict Single-Expression Bodies

**Status:** RESOLVED. Implicit `do` now works as expected in `defn`, `let`, `loop`, etc.

## 2. Numeric Type Inference and Literals

**Status:** IMPROVED. `(cast expr Type)` helps, and `usize` is implied for indices. However, type mismatches (e.g. `math/lt` between `usize` and `i64`) still require manual casting.

## 3. Range Lowering Bug (Critical)

**Status:** RESOLVED. Range lowering no longer subtracts before the first iteration; it checks bounds before executing the loop body, so unsigned `start` values are safe.

## 4. Automatic Cloning Transparency

**Status:** RESOLVED. Vectors now always clone on variable access in generated Rust (Arc clone), so first and subsequent uses are safe without explicit `core/clone`.

## 5. Main Visibility

**Status:** RESOLVED. `main` is always generated as `pub fn main` regardless of params.

## 6. Loop Semantics

The `(loop expr)` and `(while cond expr)` forms are subtle. `while` appears to lower to a loop itself. Wrapping `while` in `loop` (e.g., `(loop (while ...))`) creates an infinite loop.
*   **Status:** User error/learning curve. Use `(while ...)` directly.

## 7. Random Number State Threading

`darcy.rand` is purely functional.
*   **Status:** Requires manual threading of `rng` state, which is verbose but correct for functional paradigms.
