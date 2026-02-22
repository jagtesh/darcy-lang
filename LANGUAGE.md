# Language Guide (Living Document)

This document describes the current Lisp-like DSL that compiles to Rust. It will evolve as the language grows.

## Goals

- Lisp-like syntax with small, predictable surface area.
- Static typing with local inference and clear errors.
- Lowering to safe, fast Rust.

## Lexical Rules

- **Identifiers**: lowercase kebab-case (e.g. `total-prices`, `order-book`).
- **Qualified names**: `module/name` (module prefix + item). Module prefixes can contain dot segments, e.g. `darcy.io/dbg`.
- **Comments**:
  - Line: `; comment`
  - Block: `#| comment |#`
- **Reader conveniences**:
  - Commas are treated as whitespace.
  - `#_` discards the next form.
  - `'x` expands to `(quote x)`.
  - `` `x`` expands to `(syntax-quote x)`.
  - `~x` expands to `(unquote x)`.
  - `~@x` expands to `(unquote-splicing x)`.
  - `^meta` attaches metadata to the following form.
- **Strings**: double-quoted, e.g. `"hello"`. Escapes: `\\`, `\"`, `\n`, `\t`, `\r`.
- **Reserved keywords** (cannot be used as identifiers): `def`, `defn`, `defpub`, `defin`, `defrecord`, `defenum`, `extern`, `export`, `case`, `cond`, `and`, `or`, `when`, `if`, `do`, `loop`, `while`, `for`, `break`, `continue`, `let`, `let!`, `fn`, `call`, `type`, `cast`, `require`, `list`, `vec`, `set`, `hashset`, `range`, `range-incl`, `true`, `false`, `nil`.

## Literals

- Integers default to `i64`.
- Floats default to `f64`.
- Booleans: `true`, `false`.
- Unit: `nil` (lowered to `()`).
- Keywords: `:foo` (lowered to a `string`).
- Vectors: `[1 2 3]`.
- Maps: `{:a 1 :b 2}` (hash map literals).
- Sets: `#{1 2 3}` or `(set 1 2 3)` or `(hashset 1 2 3)` (hash sets).

## Types

- Primitives: `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`, `u32`, `u64`, `u128`, `f32`, `f64`, `bool`, `usize`, `isize`, `()`.
- Strings: `string` (lowered to `String`).
- Options: `option<T>`.
- Results: `result<T,E>`.
- Maps: `hash-map<K,V>` and `btree-map<K,V>`.
- Vectors: `vec<T>` or `Vec<T>` in DSL type annotations (e.g. `vec<i32>`). The type name is case-insensitive.
- Sets: `set<T>` or `hashset<T>`.
- Named types: structs and unions declared in the DSL or imported via modules.

## Naming and Rust Lowering

- DSL identifiers are kebab-case.
- Lowering rules:
  - Values and fields -> snake_case.
  - Types and variants -> PascalCase.
- To override Rust names for extern declarations, use `(extern "RustName" (def...))`.
- Darcy module namespaces are resolved at compile time. The generated Rust code is a flat module
  (no nested modules), so names are emitted at top level.

## Semantics and Evaluation

- Evaluation order is left-to-right within a form.
- Most forms are expressions and return a value (including `if`, `cond`, `loop`, `while`, and `for`).
- `do` is the sequencing primitive; several forms accept multiple body expressions and implicitly wrap them in `do`.
- `let` introduces new bindings; `let!` updates an existing binding and returns `()`.
- Vectors are persistent values (Arc-backed in generated Rust). Mutating operations return a new vector.
- The compiler inserts clones for non-`Copy` values as needed; you do not write `clone` for vectors.

### Calling Darcy from Rust

Compile the Darcy source to a Rust file and include it in your crate.

Darcy:

```
(defrecord order (qty i32) (price f64))
(defn total-prices [o:order]
  (* o.qty o.price))
```

Rust:

```rust
mod darcy_gen;

fn main() {
    let o = darcy_gen::Order { qty: 2, price: 3.5 };
    let total = darcy_gen::total_prices(o);
    println!("{total}");
}
```

Notes:

- Use the lowered Rust names (kebab-case -> snake_case, types -> PascalCase).
- Darcy module prefixes are not preserved in Rust output; if you want namespacing,
  place the generated file inside a Rust module.

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

- `defn` defines an internal function (lowered as `fn` in Rust).
- `export` wraps a `defn` to define an exported function (lowered as `pub fn` in Rust). Exported functions require fully known types at the boundary.
- `defn` accepts multiple body expressions (implicit `do`).
- `defpub` remains supported for compatibility, but `export` is preferred.

- Parameters are symbols, optionally annotated: `x:i32`.

### Macros

```
(defmacro twice [x]
  (list '+ x x))
```

- Macros run at compile time and return code as data.
- Macro bodies have access to a small macro-time stdlib (list/vec/hash-map/cons/concat/first/rest/nth/count/if/let/do/symbol/keyword).
- `syntax-quote` (`) supports `unquote` (`~`) and `unquote-splicing` (`~@`).
- Auto-gensym uses the `name#` suffix inside syntax-quote (e.g. `x#`).
- `gensym` generates a unique symbol (`(gensym)` or `(gensym "tmp")`).
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
- `let!` updates an existing local binding: `(let! name expr)` returns `()`.
- `let` and `fn` accept multiple body expressions (implicit `do`).

### Control Flow

```
(if cond then-expr [else-expr])
(do expr1 expr2 ... exprN)
(loop expr)
(while cond expr)
(for i (range 0 10) expr)
(break [expr])
(continue)
(cond (test expr) ... (else expr))
(mod a b)
(= a b)
(< a b) (<= a b) (> a b) (>= a b)
(& a b) (| a b)
```

- `if` without `else` returns `()`.
- `do` evaluates each expression in order and returns the last value.
- `loop`, `while`, and `for` are expressions and can `break` with a value.
- `loop`, `while`, and `for` accept multiple body expressions (implicit `do`).
- `cond` is sugar for nested `if` and must end with `else` to be total.
- `mod` is the integer remainder operator.
- `=` is equality; `<`, `<=`, `>`, `>=` are comparisons.
- `&` and `|` are bitwise integer operators.

### Ranges

```
(range start end [step])       ; end is exclusive
(range-incl start end [step])  ; end is inclusive
```

- Ranges are numeric only (ints/floats).
- Float ranges default to a `1.0` step when omitted.
- Range lowering no longer underflows for unsigned start values.

### Extern

```
(extern (defrecord file (fd i32)))
(extern "File" (defrecord file (fd i32)))
(extern (defenum io-error (not-found) (perm)))
(extern (defn write [f:file data:i32] i32))
```

- `extern` wraps `defrecord`, `defenum`, or `defn`.
- Extern functions must declare a return type and parameter types.

### Rust Interop Macros

To reduce boilerplate, the `darcy.rust` stdlib module provides macro helpers:

```
(require [darcy.rust :refer [defextern defextern-record]])

(defextern-record mnist-data "darcy_runtime::mnist::MnistData"
  [(images vec<vec<f64>>) (labels vec<vec<f64>>)])

(defextern load-edn-gz [path:string] mnist-data "darcy_runtime::mnist::load_edn_gz")
```

## Expressions

### Variables

- Symbols refer to variables, e.g. `x`.

### Type Ascription

```
(type expr Type)
```

- Forces `expr` to unify with `Type`; this is a constraint, not a runtime cast.
- Sugar for symbol-only ascription:
  - `x:Type`
  - `(x:Type)` when a long union would be hard to read inline.

### Cast

```
(cast expr Type)
```

- Runtime numeric conversion using Rust `as`.
- Source must be numeric or `bool`; target must be numeric (`i8/i16/i32/i64/i128/u8/u16/u32/u64/u128/f32/f64/usize/isize`).
- `true` casts to `1`, `false` casts to `0` for numeric targets.
- Casting to `usize` matches Rust `as` behavior (wraps/truncates on overflow).

### Numeric Overflow

- Integer overflow follows Rust semantics: debug builds panic on overflow, release builds wrap.

### Exported Functions

- `main` is always exported when defined as `(defn main ...)`.
- `main` is exported regardless of parameters, but the Rust runner expects a zero-argument entry point.
- For other functions, use:

```
(export (defn name [params] body))
```

- Exported functions require explicit, fully known parameter and return types.

### Vector Indices

- Vector indices and length values use `usize` by default.
- Index arguments accept any integer type and are converted to `usize` using Rust `as` semantics (wraps/truncates on overflow).

### Value Semantics and Sharing

- Darcy values are immutable by default.
- Vectors are persistent and share underlying storage. The compiler lowers vectors to `Arc<Vec<T>>` in Rust.
- Cloning a vector is cheap (it clones the `Arc`, not the elements).
- Operations that "modify" vectors (`darcy.vec/push`, `darcy.vec/set`, `darcy.vec/take`) return a new vector value.
- Sharing is handled automatically by the compiler; you never write `Arc` in Darcy code.

### Field Access

- Sugar: `o.qty`.
- Works for struct values and vector-of-structs (broadcasted field access).

### Method Calls (Rust Interop)

- Instance method calls:
  - `(. obj method arg ...)`
  - `(.method obj arg ...)`
- Method names allow alphanumerics, `_`, and `-` (kebab-case lowers to snake_case).
- These calls are typechecked optimistically; `len`, `is-empty`, and `push` on vectors and `len`/`is-empty` on strings add constraints, and Rust enforces the final method resolution.

### Rust Interop Declarations

- Transparent extern structs (field access allowed):

```
(extern "RustType" (defrecord rust-type (field1 i32) (field2 string)))
```

- Opaque extern types (no fields; pass through functions):

```
(extern "RustType" (defrecord rust-type))
```

- Extern function signatures:

```
(extern "rust_fn" (defn rust-fn [x:rust-type] i32))
```

### Function Calls

- Prefix form: `(+ a b)`, `(total o)`.

### Literals

- `true` / `false` are boolean literals.
- `nil` lowers to unit `()`.
- Keywords like `:status` lower to strings (e.g. `":status"`).

### Literals
- `true` / `false` are boolean literals.
- `nil` lowers to unit `()`.
- Keywords like `:status` lower to strings (e.g. `":status"`).

### Conditionals and Booleans

- `if`: `(if cond then [else])`.
- `cond`: `(cond (test expr) ... (else expr))`.
- `when`: `(when cond expr ...)` (no else; returns `nil`).
- Short-circuit:
  - `(and a b c)` evaluates left-to-right and returns a boolean.
  - `(or a b c)` evaluates left-to-right and returns a boolean.

### Match / Case

```
(case x
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
- `list` is an alias for vector literals, e.g. `(list 1 2 3)`.
- Vector helpers:
  - `(darcy.vec/new)`
  - `(darcy.vec/push v x)`
  - `(darcy.vec/repeat x n)`
  - `(darcy.vec/map f v)`
  - `(darcy.vec/map2 f a b)`
  - `(darcy.vec/fold f init v)`
  - `(darcy.vec/take v n)`
- Utility:
  - `(darcy.core/clone x)` clones any `Clone`-able value.

#### Broadcasting

- Vector-scalar numeric ops are supported:
  - `(* [1 2 3] 2)`
  - `(+ 2 [1 2 3])`
- Vector-vector numeric ops are not supported.

### Maps

- Hash map literal:
  - `{:a 1 :b 2}`
  - `(darcy.hash-map/new [:a 1] [:b 2])`
- B-tree map literal:
  - `(darcy.btree-map/new [:a 1] [:b 2])`
- Empty map requires annotation:
  - `(darcy.hash-map/new<string,i32>)`

### Sets

- Set literals:
  - `#{1 2 3}`
  - `#{1 2 3}` or `(hashset 1 2 3)`
- Empty set requires annotation:
  - `(set<i32>)`

## Modules and Imports

Modules are files addressed by dot-separated symbols and brought into scope with `require` (Clojure-style vectors).

### Module Paths

- File `std/io.dsl` is imported as:

```
(require [darcy.io])
```

Dots map to directories (`darcy.io` -> `darcy/io.dsl`).

Module search paths are:

- Any `--lib`/`-L` paths passed on the CLI (in order)
- The directory of the input file
- The current working directory

### Standard Library Highlights

- Math helpers: `darcy.math/exp`, `darcy.math/gt`, `darcy.math/lt`, `darcy.math/eq`.
- Tensor helpers: `darcy.tensor/*` (vec/matrix ops).
- NN helpers: `darcy.nn/*` (linear layer, softmax, training helpers).

### Import Forms

```
(require [darcy.io])
(require [darcy.io :as io])
(require [darcy.io :refer [dbg read]])
(require [darcy.io :refer :all])
```

- `require` keeps names under their module unless `:refer` is used.
- `:as` creates an alias prefix.
- `:refer [name ...]` imports named items into the current namespace.
- `:refer :all` imports everything into the current namespace.
- Use qualified call heads for direct module access:
  - `(darcy.io/dbg 1)`
  - `(io/dbg 1)`

### Built-in Modules (MVP)

- `darcy.io`: `dbg`
- `darcy.math`: `abs`, `min`, `max`, `clamp`, `exp`, `gt`, `lt`, `eq`
- `darcy.core`: `clone`
- `darcy.vec`: `len`, `is-empty`, `get`, `set`

Notes:
- `darcy.vec/get` returns the element (cloned).
- `darcy.vec/set` clones and updates a copy, returning `()`. Local assignment is available via `(let! name expr)`.
- `darcy.string`: `len`, `is-empty`, `trim`, `split`, `join`
- `darcy.fmt`: `format`, `pretty`, `print`, `println`
- `darcy.option`: `some`, `none`, `is-some`, `is-none`, `unwrap`, `unwrap-or`
- `darcy.result`: `ok`, `err`, `is-ok`, `is-err`, `unwrap`, `unwrap-or`
- `darcy.hash-map`: `new`, `len`, `is-empty`, `get`, `contains`, `insert`, `remove`
- `darcy.btree-map`: `new`, `len`, `is-empty`, `get`, `contains`, `insert`, `remove`
- `darcy.rand`: `seed`, `rand-f64`, `rand-normal`, `rand-range`

### Resolution Rules

- If a symbol is `module/name`, it resolves through the module prefix.
- Otherwise, it resolves to local definitions, then `require` imports.

## Type Inference

The current system is monomorphic and unification-based (HM-style without generalization).

- Parameters without annotations are inferred from usage:
  - literals
  - struct field access
  - calls to functions with known types
  - numeric operators
- Method calls are mostly opaque to inference today; `len`, `is-empty`, and `push` on vectors and `len`/`is-empty` on strings contribute constraints.
- If a concrete type is required and cannot be inferred (e.g., externs or struct fields), compilation fails with a message like:
  - `cannot infer type for parameter 'x': no constraints. Add annotation like x:Type`
- Unconstrained function parameters are treated as generic type parameters in Rust output, with inferred bounds from usage (e.g., `Clone`, `Copy`, `Debug`, `PartialEq`, `PartialOrd`, numeric ops).
- When generic method calls need it, the compiler emits internal traits like `__DarcyLen` or `__DarcyPush` in the generated Rust.
- Numeric operators default unresolved numeric variables to `i64`.
- Functions inferred to use parameters only in borrow positions (field access or by-ref calls) are lowered to borrowed Rust parameters (`&T`).
- When a non-copy value is reused after a move, the compiler inserts an auto-clone and emits a warning.

## Polymorphism

Darcy supports three polymorphism modes:

- Default (trait-bound generics): unconstrained parameters become Rust generics with inferred bounds.
- Union/enum polymorphism (opt-in): use union types in params, either with `defunion` or inline `|` types (e.g., `x:i32|i64|f32|f64`).
- Specialized monomorphs (opt-in): `defn.specialize` generates concrete variants based on actual Darcy call sites.

Return types are inferred from the body. For numeric ops, the left operand determines the result type. When the left operand is a union, the return is that union unless explicitly annotated.

## Compiler Pipeline

The compiler uses a multi-stage pipeline with a feedback pass that combines rustc diagnostics and rust-analyzer semantic inference.

Stages:

1. Parse + macro expansion
2. Typecheck + borrow/move heuristics
3. Lower to Rust IR
4. Emit Rust (pass 1)
5. Feedback (parallel):
   - rustc diagnostics: `cargo check --message-format=json`
   - rust-analyzer diagnostics via `load-cargo` + `ide::AnalysisHost`
6. Merge feedback into `FeedbackHints` (rustc wins on conflicts)
7. Re-lower + emit Rust (pass 2)

Merge policy (deterministic):
- rustc errors/warnings take precedence over RA hints
- RA fills in missing trait bounds or expected types when rustc is silent
- conflicts are resolved by keeping rustc-derived constraints

This feedback stage is opt-in via `--feedback`. The RA pass can add trait bounds or expected types
even when rustc is silent, improving polymorphism and type propagation without changing surface syntax.

## Builtins

- Numeric operators: `+ - * /`
- `darcy.io/dbg` is currently a builtin returning `()`.

## Program Entry

- A zero-argument `main` is treated as the entry point:

```
(defn main []
  (darcy.io/dbg 42))
```

## Example

```
(defrecord order (qty i32) (price f64))

(defn total [o]
  (* o.qty o.price))

(defn main []
  (darcy.io/dbg (total (order 2 3.5))))
```

## Current Limitations

- No user-defined generics or traits.
- Polymorphism is limited to compiler-inferred traits; user-defined trait bounds are not supported.
- Macro system is minimal (no hygiene or compile-time modules).
- Borrowing/ownership uses lightweight heuristics, not full Rust borrow checking.
- `darcy.io/dbg` is still a builtin (planned to move to the standard library).

## Roadmap Notes

- Standard library modules under the `darcy.*` namespace.
- `darcy.rand`: RNG helpers (seed, rand-f64, rand-normal, rand-range)
- Move I/O and other runtime functions fully out of builtins.
- More inference coverage and better diagnostics.
