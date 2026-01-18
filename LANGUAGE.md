# Language Guide (Living Document)

This document describes the current Lisp-like DSL that compiles to Rust. It will evolve as the language grows.

## Goals

- Lisp-like syntax with small, predictable surface area.
- Static typing with local inference and clear errors.
- Lowering to safe, fast Rust.

## Lexical Rules

- **Identifiers**: lowercase kebab-case (e.g. `total-prices`, `order-book`).
- **Qualified names**: `module/name` (module prefix + item). Module prefixes can contain dot segments, e.g. `std.io/dbg`.
- **Comments**:
  - Line: `; comment`
  - Block: `#| comment |#`
- **Strings**: double-quoted, e.g. `"hello"`. Escapes: `\\`, `\"`, `\n`, `\t`, `\r`.
- **Reserved keywords** (cannot be used as identifiers): `def`, `defn`, `defin`, `defstruct`, `defunion`, `defrecord`, `defenum`, `extern`, `match`, `case`, `if`, `do`, `loop`, `while`, `for`, `break`, `continue`, `let`, `fn`, `call`, `use`, `require`, `open`, `vec`, `range`, `range-incl`, `true`, `false`, `nil`.

## Literals

- Integers default to `i32`.
- Floats default to `f64`.
- Booleans: `true`, `false`.
- Unit: `nil` (lowered to `()`).
- Keywords: `:foo` (lowered to a `string`).
- Vectors: `[1 2 3]`.
- Maps: `{:a 1 :b 2}` (hash map literals).

## Types

- Primitives: `i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`, `usize`, `isize`, `()`.
- Strings: `string` (lowered to `String`).
- Options: `option<T>`.
- Results: `result<T,E>`.
- Maps: `hashmap<K,V>` and `btreemap<K,V>`.
- Vectors: `vec<T>` or `Vec<T>` in DSL type annotations (e.g. `vec<i32>`). The type name is case-insensitive.
- Named types: structs and unions declared in the DSL or imported via modules.

## Naming and Rust Lowering

- DSL identifiers are kebab-case.
- Lowering rules:
  - Values and fields -> snake_case.
  - Types and variants -> PascalCase.
- To override Rust names for extern declarations, use `(extern "RustName" (def...))`.

## Top-Level Forms

### Structs (Records)

```
(defrecord order
  (qty u32)
  (price f64))
```

- Field types can be omitted when inferable: `(defrecord order (qty) (price))`.

### Unions (discriminated unions)

```
(defenum result
  (ok (value i32))
  (err (code i32) (msg i32)))
```

- Variant field types can be omitted when inferable: `(defenum result (ok (value)) (err (code) (msg)))`.

### Functions

```
(defn total [o:order]
  (* o.qty o.price))
```

- Parameters are symbols, optionally annotated: `x:i32`.
- Parameter types are inferred when possible; otherwise they must be annotated.

### Definitions (global values)

```
(def base 10)
```

- `def` creates an immutable, lazily initialized global value.
- Global `def` names cannot be shadowed by `let`, `fn` parameters, or `defn` parameters.

### Inline (displaced closures)

```
(defin inc [x]
  (+ x 1))
```

- Inlines are expanded at the call site.
- Free variables are resolved in the caller scope.

### Lambdas and Local Bindings

```
(let [x 1 y 2]
  (call (fn [z] (+ z x)) y))
```

- `let` binds local variables and evaluates the body expression.
- Bindings can be written as pairs: `[(x 1) (y 2)]` or flat pairs: `[x 1 y 2]`.
- `fn` creates a closure; `call` invokes a value as a function.

### Control Flow

```
(if cond then-expr [else-expr])
(do expr1 expr2 ... exprN)
(loop expr)
(while cond expr)
(for i (range 0 10) expr)
(break [expr])
(continue)
```

- `if` without `else` returns `()`.
- `do` evaluates each expression in order and returns the last value.
- `loop`, `while`, and `for` are expressions and can `break` with a value.
- Loop bodies are single expressions (no implicit blocks yet).

### Ranges

```
(range start end [step])       ; end is exclusive
(range-incl start end [step])  ; end is inclusive
```

- Ranges are numeric only (ints/floats).

### Extern

```
(extern (defrecord file (fd i32)))
(extern "File" (defrecord file (fd i32)))
(extern (defenum io-error (not-found) (perm)))
(extern (defn write [f:file data:i32] i32))
```

- `extern` wraps `defrecord`/`defstruct`, `defenum`/`defunion`, or `defn`.
- Extern functions must declare a return type and parameter types.

## Expressions

### Variables

- Symbols refer to variables, e.g. `x`.

### Field Access

- Sugar: `o.qty`.
- Works for struct values and vector-of-structs (broadcasted field access).

### Function Calls

- Prefix form: `(+ a b)`, `(total o)`.

### Literals

- `true` / `false` are boolean literals.
- `nil` lowers to unit `()`.
- Keywords like `:status` lower to strings (e.g. `":status"`).

### Match / Case

```
(case x
  (ok (value v) v)
  (err (code c) c)
  (_ 0))
```

- Patterns are variant names with bindings.
- Exhaustive by default; `_` is a wildcard.
- `match` is accepted as an alias of `case`.

### Vectors

- Literals: `[1 2 3]`.
- Typed literals: `(vec<i32> 1 2 3)`.
- Empty vector requires annotation: `(vec<i32>)`.

#### Broadcasting

- Vector-scalar numeric ops are supported:
  - `(* [1 2 3] 2)`
  - `(+ 2 [1 2 3])`
- Vector-vector numeric ops are not supported.

### Maps

- Hash map literal:
  - `{:a 1 :b 2}`
  - `(core.hashmap/new ("a" 1) ("b" 2))`
  - `{ "a" 1 "b" 2 }`
- B-tree map literal:
  - `(core.btreemap/new ("a" 1) ("b" 2))`
- Empty map requires annotation:
  - `(core.hashmap/new<string,i32>)`

## Modules and Imports

Modules are files addressed by dot-separated symbols and brought into scope with `use`/`require` or `open`.

### Module Paths

- File `std/io.dsl` is imported as:

```
(use std.io)
```

Dots map to directories (`std.io` -> `std/io.dsl`).

Module search paths are:

- Any `--lib`/`-L` paths passed on the CLI (in order)
- The directory of the input file
- The current working directory

### Import Forms

```
(use std.io)
(require std.io)
(use std.io :as io)
(use std.io :only (dbg read))
(open std.io)
```

- `use` keeps names under their module unless `:only` is used.
- `require` is accepted as an alias of `use`.
- `:as` creates an alias prefix.
- `:only` imports named items into the current namespace.
- `open` imports everything into the current namespace.
- Use qualified call heads for direct module access:
  - `(std.io/dbg 1)`
  - `(io/dbg 1)`

### Built-in Modules (MVP)

- `std.io`: `dbg`
- `core.num`: `abs`, `min`, `max`, `clamp`
- `core.vec`: `len`, `is-empty`, `get`, `set`

Notes:
- `core.vec/get` returns the element (cloned).
- `core.vec/set` currently clones and updates a copy, returning `()`. This will evolve when mutation/assignment is added.
- `core.str`: `len`, `is-empty`, `trim`, `split`, `join`
- `core.fmt`: `dbg`, `format`, `pretty`, `print`, `println`
- `core.option`: `some`, `none`, `is-some`, `is-none`, `unwrap`, `unwrap-or`
- `core.result`: `ok`, `err`, `is-ok`, `is-err`, `unwrap`, `unwrap-or`
- `core.hashmap`: `new`, `len`, `is-empty`, `get`, `contains`, `insert`, `remove`
- `core.btreemap`: `new`, `len`, `is-empty`, `get`, `contains`, `insert`, `remove`

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
- `dbg` is currently a builtin returning `()`.

## Program Entry

- A zero-argument `main` is treated as the entry point:

```
(defn main []
  (dbg 42))
```

## Example

```
(defrecord order (qty i32) (price f64))

(defn total [o]
  (* o.qty o.price))

(defn main []
  (dbg (total (order 2 3.5))))
```

## Current Limitations

- No user-defined generics or traits.
- No macros.
- Borrowing/ownership modeled only via Rust lowering.
- `dbg` is still a builtin (planned to move to std).

## Roadmap Notes

- Standard library modules (`std/*`) with `core` re-exports.
- Move I/O and other runtime functions out of builtins.
- More inference coverage and better diagnostics.
