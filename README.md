# lisp2rust MVP

This is a tiny proof-of-concept "Elm-ish typed Lisp" frontend that lowers to Rust.

What it supports (MVP):
 - `(defstruct name (field type) ...)`
 - `(defn name [params] expr)`
 - `(def name expr)`
   - params are symbols like `o:order` (type annotation required unless inference is unambiguous)
- Expressions:
  - numeric literals (ints default to i32, floats to f64)
  - if: `(if cond then [else])`
  - sequencing: `(do expr1 expr2 ... exprN)`
  - local bindings: `(let [x 1 y 2] expr)`
  - closures: `(fn [x] expr)` and `(call f arg ...)`
  - loops: `(loop expr)`, `(while cond expr)`, `(for i (range 0 10) expr)`
  - loop control: `(break [expr])`, `(continue)`
  - variables
  - field access sugar: `o.qty`
  - binary ops: `+ - * /` in prefix form: `(* a b)`
  - unions + match: `(defunion name (variant (field Type) ...) ...)`, `(match x (variant (field v) expr) (_ expr))`
  - vectors: `[1 2 3]`, `(vec<i32> 1 2 3)`
  - vector index: `(core.vec/get v i)`, `(core.vec/set v i x)`
  - debug print: `(dbg expr)`
  - formatted print: `(core.fmt/print "x={}\n" x)`, `(core.fmt/println "x={}" x)`
  - extern wrapper: `(extern (defstruct ...))`, `(extern (defunion ...))`, `(extern (defn name [params] RetType))`
  - inline expansion: `(defin name [params] expr)`
  - comments: `; line` and `#| block |#`
  - modules: `(use std.io)`, `(use std.io :as io)`, `(use std.io :only (dbg))`, `(open std.io)`

What it does NOT support yet:
- borrowing/ownership surface syntax
- generics / traits
- closures as return types
- macros
- helpful multi-span diagnostics (only 1 span)

## Try it

From this folder:

```bash
cargo run -p dslc -- examples/ok.dsl > out.rs
cat out.rs
```

Add a stdlib search path:

```bash
cargo run -p dslc -- --lib stdlib examples/ok.dsl
```

## Modules quickstart

```lisp
(use std.io :as io)

(defn main []
  (io/dbg 42))
```

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

- `examples/strategy_stage1.dsl` uses structs, unions, match, vectors, and dbg.
- `examples/strategy_stage2.dsl` introduces extern types/functions and regime logic.
- `examples/strategy_stage3.dsl` sketches multi-asset portfolio logic with broadcasting.

## Notes

The intent is: your compiler does the "clarity layer" (shape + local type inference + good errors),
and Rust remains the final checker for deep lifetime / trait issues.

Naming: DSL identifiers are lowercase kebab-case. They are normalized to Rust identifiers during lowering
(types and variants become PascalCase; values become snake_case). Use `(extern "RustName" (def...))`
to override the Rust name for extern declarations.

See `LANGUAGE.md` for the living language guide.
