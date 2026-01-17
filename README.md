# lisp2rust MVP

This is a tiny proof-of-concept "Elm-ish typed Lisp" frontend that lowers to Rust.

What it supports (MVP):
 - `(defstruct name (field type) ...)`
 - `(defn name [params] expr)`
   - params are symbols like `o:order` (type annotation required unless inference is unambiguous)
- Expressions:
  - numeric literals (ints default to i32, floats to f64)
  - variables
  - field access sugar: `o.qty`
  - binary ops: `+ - * /` in prefix form: `(* a b)`
  - unions + match: `(defunion name (variant (field Type) ...) ...)`, `(match x (variant (field v) expr) (_ expr))`
  - vectors: `[1 2 3]`, `(vec<i32> 1 2 3)`
  - print: `(print expr)`
  - extern wrapper: `(extern (defstruct ...))`, `(extern (defunion ...))`, `(extern (defn name [params] RetType))`
  - comments: `; line` and `#| block |#`
  - modules: `(use "std/io")`, `(use "std/io" :as io)`, `(use "std/io" :only (print))`, `(open "std/io")`

What it does NOT support yet:
- borrowing/ownership surface syntax
- generics / traits
- function calls beyond numeric ops
- pattern matching
- macros
- helpful multi-span diagnostics (only 1 span)

## Try it

From this folder:

```bash
cargo run -p dslc -- examples/ok.dsl > out.rs
cat out.rs
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

To compare against CEL, enable the `cel` feature. The harness uses the `cel` crate to evaluate a
simple expression with a custom `last_sma` function.

Try the error cases:

```bash
cargo run -p dslc -- examples/ambiguous.dsl
cargo run -p dslc -- examples/missing_field.dsl
```

## Strategy examples (staged)

- `examples/strategy_stage1.dsl` uses structs, unions, match, vectors, and print.
- `examples/strategy_stage2.dsl` introduces extern types/functions and regime logic.
- `examples/strategy_stage3.dsl` sketches multi-asset portfolio logic with broadcasting.

## Notes

The intent is: your compiler does the "clarity layer" (shape + local type inference + good errors),
and Rust remains the final checker for deep lifetime / trait issues.

Naming: DSL identifiers are lowercase kebab-case. They are normalized to Rust identifiers during lowering
(types and variants become PascalCase; values become snake_case). Use `(extern "RustName" (def...))`
to override the Rust name for extern declarations.

See `LANGUAGE.md` for the living language guide.
