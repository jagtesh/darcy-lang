# lisp2rust MVP

This is a tiny proof-of-concept "Elm-ish typed Lisp" frontend that lowers to Rust.

What it supports (MVP):
 - `(defrecord name (field type) ...)`
 - `(defn name [params] expr)`
 - `(defmacro name [params] expr)`
 - `(def name expr)`
   - params are symbols like `o:order` (type annotation required unless inference is unambiguous)
- Expressions:
  - numeric literals (ints default to i64, floats to f64)
  - if: `(if cond then [else])`
  - when: `(when cond expr ...)`
  - short-circuit: `(and a b)`, `(or a b)`
  - sequencing: `(do expr1 expr2 ... exprN)`
  - local bindings: `(let [x 1 y 2] expr)`
  - closures: `(fn [x] expr)` and `(call f arg ...)`
  - loops: `(loop expr)`, `(while cond expr)`, `(for i (range 0 10) expr)`
  - cond: `(cond (test expr) ... (else expr))`
  - loop control: `(break [expr])`, `(continue)`
  - variables
  - field access sugar: `o.qty`
  - method calls: `(. obj method arg ...)`, `(.method obj arg ...)`
  - binary ops: `+ - * /` in prefix form: `(* a b)`
  - unions + case: `(defenum name (variant (field Type) ...) ...)`, `(case x (variant (field v) expr) (_ expr))`
  - booleans, nil, and keywords: `true`, `false`, `nil`, `:key`
  - vectors: `[1 2 3]`, `(vec<i32> 1 2 3)`
  - type ascription: `(type expr Type)`
  - list alias: `(list 1 2 3)`
  - vector index: `(darcy.vec/get v i)`, `(darcy.vec/set v i x)`
  - vector helpers: `(darcy.vec/new)`, `(darcy.vec/push v x)`, `(darcy.vec/repeat x n)`, `(darcy.vec/map f v)`, `(darcy.vec/map2 f a b)`, `(darcy.vec/fold f init v)`, `(darcy.vec/take v n)`
  - cloning: `(darcy.core/clone x)`
  - inferred borrowing: params used only in borrow positions (field access or by-ref calls) lower to `&T`
  - auto-clone on reuse after move (compiler emits a warning)
  - map literals: `{:a 1 :b 2}`, `(darcy.hash-map/new [:a 1] [:b 2])`
  - set literals: `#{1 2 3}`, `(set 1 2 3)`, `(hashset 1 2 3)`
  - local assignment: `(let! name expr)` returns `()`
  - debug print: `(darcy.io/dbg expr)`
  - formatted print: `(darcy.fmt/print (darcy.fmt/format x))`, `(darcy.fmt/println (darcy.fmt/format x))`
  - math helpers: `(darcy.math/exp x)`, `(darcy.op/gt a b)`, `(darcy.op/lt a b)`, `(darcy.op/eq a b)`
  - extern wrapper: `(extern (defrecord ...))`, `(extern (defenum ...))`, `(extern (defn name [params] RetType))`
  - rust interop macros: `(require [darcy.rust :refer [defextern defextern-record]])`
  - stdlib modules: `darcy.tensor`, `darcy.nn`, `darcy.mnist`
  - inline expansion: `(defin name [params] expr)`
  - comments: `; line` and `#| block |#`
  - reader: commas are whitespace, `#_` discards the next form
  - quote: `'x` expands to `(quote x)`, `` `x`` expands to `(syntax-quote x)`
  - unquote: `~x` and `~@x` inside syntax-quote
  - metadata: `^meta` attaches metadata to the next form (currently ignored in lowering)
  - modules: `(require [darcy.io])`, `(require [darcy.io :as io])`, `(require [darcy.io :refer [dbg]])`, `(require [darcy.io :refer :all])`

What it does NOT support yet:
- explicit borrowing/ownership surface syntax
- full Rust-style borrow checking (current system uses heuristics + auto-clone warnings)
- explicit user-defined generics / traits (compiler infers generic bounds for unconstrained functions)
- closures as return types
- helpful multi-span diagnostics (only 1 span)

## Try it

From this folder:

```bash
cargo run -p dslc -- examples/ok.dsl > out.rs
cat out.rs
```

Add a stdlib search path:

```bash
cargo run -p dslc -- --lib crates/darcy-stdlib/darcy examples/ok.dsl
```

Enable the feedback pass (two-stage emit + rustc diagnostics):

```bash
cargo run -p dslc -- --feedback examples/ok.dsl
```

Run with Rust interop (uses `darcy-runtime` automatically when referenced):

```bash
cargo run -p dslc -- --lib crates/darcy-stdlib/darcy run examples/mnist.dsl --runtime
```

Note: the compiler may emit warnings when it auto-clones values to avoid move errors.

## Modules quickstart

```lisp
(require [darcy.io :as io])

(defn main []
  (io/dbg 42))
```

## Cargo-integrated workflow

Use the Cargo subcommand to scaffold a Rust project with Darcy integration:

```bash
cargo darcy init my-app
cd my-app
cargo run
```

This creates a normal Cargo crate with `build.rs` and a `darcy/` folder. The
build step compiles Darcy sources into Rust and includes them automatically.
Darcy's stdlib comes from the `darcy-stdlib` crate by default.
The template enables the `darcy-compiled` feature by default, which generates
and re-exports the Darcy module at the crate root.

If you're developing from this repo, set `DARCY_SDK` to the repo root so
`cargo darcy init` uses path dependencies to `darcy-build` and `darcy-stdlib`:

```bash
export DARCY_SDK=/path/to/darcy-lang
```

## Calling Darcy from Rust

For a standard, automated workflow, use the `darcy-build` crate in `build.rs` so
Darcy sources are compiled during `cargo build`.

```toml
[build-dependencies]
darcy-build = { path = "path/to/darcy-lang/crates/darcy-build" }
darcy-stdlib = { path = "path/to/darcy-lang/crates/darcy-stdlib" }
```

```rust
// build.rs
fn main() {
    darcy_build::Builder::new("darcy/main.dsl")
        .lib_path("darcy")
        .stdlib_path(darcy_stdlib::stdlib_dir())
        .compile()
        .expect("darcy compile failed");
}
```

```rust
// src/main.rs
mod darcy_gen {
    include!(concat!(env!("OUT_DIR"), "/darcy_gen.rs"));
}

fn main() {
    let o = darcy_gen::Order { qty: 2, price: 3.5 };
    let total = darcy_gen::total_prices(o);
    println!("{total}");
}
```

`darcy-build` looks for the stdlib at `DARCY_STDLIB` first, then falls back to the
repo's `crates/darcy-stdlib/darcy/` directory (when used via the Darcy workspace or a path/git dependency).

If you want one-off compilation, you can still compile a Darcy file to Rust and
include it as a module:

```bash
cargo run -p dslc -- examples/ok.dsl > src/darcy_gen.rs
```

Names are lowered to Rust identifiers (kebab-case -> snake_case, types -> PascalCase).
Darcy module prefixes are compile-time only; the generated Rust is flat.

## Benchmark harness

There is a small, dependency-free benchmark stub at `dslc/src/bin/bench.rs` that times a
simple moving-average strategy. To run it:

```bash
cargo run -p dslc --bin bench -- --iters 200000
```

There is also a typecheck/inference benchmark that reports total and per-iteration time:

```bash
cargo run -p dslc --bin bench_typecheck -- --iters 10000 --save bench/typecheck.json
cargo run -p dslc --release --bin bench_typecheck -- --iters 10000 --save bench/typecheck.release.json
```

You can save history, compare, and update baselines:

```bash
cargo run -p dslc --bin bench_typecheck -- --iters 10000 --save-dir bench/history --label main
cargo run -p dslc --bin bench_typecheck -- compare --baseline bench/baseline.json --candidate bench/typecheck.release.json --max-regression-pct 5
cargo run -p dslc --bin bench_typecheck -- update --baseline bench/baseline.json --candidate bench/typecheck.release.json
```

Makefile workflow (release builds, timestamped history, and regression checks):

```bash
make bench-check
make bench-accept
```

To compare against CEL, enable the `cel` feature. The harness uses the `cel` crate to evaluate a
simple expression with a custom `last_sma` function.

Try the error cases:

```bash
cargo run -p dslc -- examples/ambiguous.dsl
cargo run -p dslc -- examples/missing_field.dsl
```

## Strategy examples (staged)

- `examples/strategy_stage1.dsl` uses structs, unions, case, vectors, and dbg.
- `examples/strategy_stage2.dsl` introduces extern types/functions and regime logic.
- `examples/strategy_stage3.dsl` sketches multi-asset portfolio logic with broadcasting.

## Compiler pipeline

`dslc` emits Rust in two passes when `--feedback` is enabled. The feedback stage runs `cargo check`
and rust-analyzer diagnostics in parallel on the first Rust output, merges trait-bound hints, and
re-emits Rust with refined generics.

```mermaid
flowchart TD
    A[Parse + expand] --> B[Typecheck]
    B --> C[Lower to Rust IR]
    C --> D[Emit Rust (pass 1)]
    D --> E{--feedback?}
    E -->|no| F[Emit Rust (final)]
    E -->|yes| G[cargo check + JSON diagnostics]
    E -->|yes| H[rust-analyzer diagnostics]
    G --> I[Feedback hints]
    H --> I
    I --> J[Apply hints to typed IR]
    J --> K[Emit Rust (pass 2)]
    K --> F
```

Rustc remains the source of truth for hard errors; RA provides additional type/trait insights even
when code compiles.

## Notes

The intent is: your compiler does the "clarity layer" (shape + local type inference + good errors),
and Rust remains the final checker for deep lifetime / trait issues.

Naming: DSL identifiers are lowercase kebab-case. They are normalized to Rust identifiers during lowering
(types and variants become PascalCase; values become snake_case). Use `(extern "RustName" (def...))`
to override the Rust name for extern declarations.

See `LANGUAGE.md` for the living language guide.
