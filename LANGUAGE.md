# Language Guide (Living Document)

This document describes the current Lisp-like DSL that compiles to Rust. It will evolve as the language grows.

## Goals

- Lisp-like syntax with small, predictable surface area.
- Static typing with local inference and clear errors.
- Lowering to safe, fast Rust.

## Lexical Rules

- **Identifiers**: lowercase kebab-case (e.g. `total-prices`, `order-book`).
- **Qualified names**: `module/name` (module prefix + item). Module prefixes can contain dot segments, e.g. `std.io/print`.
- **Comments**:
  - Line: `; comment`
  - Block: `#| comment |#`
- **Strings**: double-quoted, e.g. `"hello"`.
- **Reserved keywords** (cannot be used as identifiers): `defn`, `defstruct`, `defunion`, `extern`, `match`, `use`, `open`, `vec`.

## Literals

- Integers default to `i32`.
- Floats default to `f64`.
- Vectors: `[1 2 3]`.

## Types

- Primitives: `i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`, `usize`, `isize`, `()`.
- Strings: `string` (lowered to `String`).
- Vectors: `vec<T>` or `Vec<T>` in DSL type annotations (e.g. `vec<i32>`). The type name is case-insensitive.
- Named types: structs and unions declared in the DSL or imported via modules.

## Naming and Rust Lowering

- DSL identifiers are kebab-case.
- Lowering rules:
  - Values and fields -> snake_case.
  - Types and variants -> PascalCase.
- To override Rust names for extern declarations, use `(extern "RustName" (def...))`.

## Top-Level Forms

### Structs

```
(defstruct order
  (qty u32)
  (price f64))
```

### Unions (discriminated unions)

```
(defunion result
  (ok (value i32))
  (err (code i32) (msg i32)))
```

### Functions

```
(defn total [o:order]
  (* o.qty o.price))
```

- Parameters are symbols, optionally annotated: `x:i32`.
- Parameter types are inferred when possible; otherwise they must be annotated.

### Extern

```
(extern (defstruct file (fd i32)))
(extern "File" (defstruct file (fd i32)))
(extern (defunion io-error (not-found) (perm)))
(extern (defn write [f:file data:i32] i32))
```

- `extern` wraps `defstruct`, `defunion`, or `defn`.
- Extern functions must declare a return type and parameter types.

## Expressions

### Variables

- Symbols refer to variables, e.g. `x`.

### Field Access

- Sugar: `o.qty`.
- Works for struct values and vector-of-structs (broadcasted field access).

### Function Calls

- Prefix form: `(+ a b)`, `(total o)`.

### Match

```
(match x
  (ok (value v) v)
  (err (code c) c)
  (_ 0))
```

- Patterns are variant names with bindings.
- Exhaustive by default; `_` is a wildcard.

### Vectors

- Literals: `[1 2 3]`.
- Typed literals: `(vec<i32> 1 2 3)`.
- Empty vector requires annotation: `(vec<i32>)`.

#### Broadcasting

- Vector-scalar numeric ops are supported:
  - `(* [1 2 3] 2)`
  - `(+ 2 [1 2 3])`
- Vector-vector numeric ops are not supported.

## Modules and Imports

Modules are files addressed by path strings and brought into scope with `use` or `open`.

### Module Paths

- File `std/io.dsl` is imported as:

```
(use "std/io")
```

### Import Forms

```
(use "std/io")
(use "std/io" :as io)
(use "std/io" :only (print read))
(open "std/io")
```

- `use` keeps names under their module unless `:only` is used.
- `:as` creates an alias prefix.
- `:only` imports named items into the current namespace.
- `open` imports everything into the current namespace.
- Use qualified call heads for direct module access:
  - `(std.io/print 1)`
  - `(io/print 1)`

### Built-in Modules (MVP)

- `std/io`: `print`
- `core/num`: `abs`, `min`, `max`, `clamp`
- `core/vec`: `len`, `is-empty`
- `core/str`: `len`, `is-empty`, `trim`, `split`, `join`

### Resolution Rules

- If a symbol is `module/name`, it resolves through the module prefix.
- Otherwise, it resolves to local definitions, then `use`/`open` imports.

## Type Inference

The current system is monomorphic and unification-based (HM-style without generalization).

- Parameters without annotations are inferred from usage:
  - literals
  - struct field access
  - calls to functions with known types
  - numeric operators
- If the type cannot be inferred, compilation fails with a message like:
  - `cannot infer type for parameter 'x': no constraints. Add annotation like x:Type`
- Numeric operators require concrete numeric types; `(+ x y)` without constraints is rejected.

## Builtins

- Numeric operators: `+ - * /`
- `print` is currently a builtin returning `()`.

## Program Entry

- A zero-argument `main` is treated as the entry point:

```
(defn main []
  (print 42))
```

## Example

```
(defstruct order (qty i32) (price f64))

(defn total [o]
  (* o.qty o.price))

(defn main []
  (print (total (order 2 3.5))))
```

## Current Limitations

- No user-defined generics or traits.
- No macros.
- Borrowing/ownership modeled only via Rust lowering.
- `print` is still a builtin (planned to move to std).

## Roadmap Notes

- Standard library modules (`std/*`) with `core` re-exports.
- Move I/O and other runtime functions out of builtins.
- More inference coverage and better diagnostics.
