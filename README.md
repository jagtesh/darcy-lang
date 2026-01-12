# lisp2rust MVP

This is a tiny proof-of-concept "Elm-ish typed Lisp" frontend that lowers to Rust.

What it supports (MVP):
- `(defstruct Name (field Type) ...)`
- `(defn name [params] expr)`
  - params are symbols like `o:Order` (type annotation required unless inference is unambiguous)
- Expressions:
  - numeric literals (ints default to i32, floats to f64)
  - variables
  - field access sugar: `o.qty`
  - binary ops: `+ - * /` in prefix form: `(* a b)`

What it does NOT support yet:
- borrowing/ownership surface syntax
- generics / traits
- function calls beyond numeric ops
- pattern matching
- modules
- macros
- helpful multi-span diagnostics (only 1 span)

## Try it

From this folder:

```bash
cargo run -p dslc -- examples/ok.dsl > out.rs
cat out.rs
```

Try the error cases:

```bash
cargo run -p dslc -- examples/ambiguous.dsl
cargo run -p dslc -- examples/missing_field.dsl
```

## Notes

The intent is: your compiler does the "clarity layer" (shape + local type inference + good errors),
and Rust remains the final checker for deep lifetime / trait issues.
