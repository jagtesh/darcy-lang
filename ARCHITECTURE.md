# Architecture

This compiler is organized as a small set of explicit, end-to-end phases. Each phase has a
single input and output type so changes remain local and easy to review.

Pipeline
- Lexing: `lexer::lex` transforms source text into `Vec<Tok>`.
- Parsing: `parser::Parser` transforms tokens into `Vec<Sexp>`.
- Reader/macros: `reader::Reader` + `macro_expand::expand_program` expand reader macros and syntax quotes.
- AST: `ast::{parse_toplevel}` transforms sexps into `Vec<Top>`.
- Modules/inline: `module::compile_modules` resolves `require` and `typecheck::expand_inline_tops` expands `defin`.
- Typecheck: `typecheck::typecheck_tops` builds a `TypecheckedProgram` with typed functions.
- Ownership heuristics: inferred borrow-by-default signatures + auto-clone plan computed in `typecheck`.
- Lowering: `lower::lower_program` converts the pipeline output into Rust source (applies borrow/auto-clone plan).

Entrypoints
- Library API: `dslc::analyze` and `dslc::compile` in `dslc/src/lib.rs`.
- CLI: `dslc/src/main.rs` reads a file and prints Rust output, plus warnings (auto-clone) if present.

Directory map
- `dslc/src/lexer.rs` tokenization and spans.
- `dslc/src/parser.rs` s-expression parsing.
- `dslc/src/reader.rs` reader macros and map/set literal parsing.
- `dslc/src/macro_expand.rs` macro expansion.
- `dslc/src/ast.rs` AST types and syntax-level parsing helpers.
- `dslc/src/typecheck.rs` type inference and typed program construction.
- `dslc/src/typed.rs` typed AST helpers.
- `dslc/src/lower.rs` Rust codegen.
- `dslc/src/pipeline.rs` glue for end-to-end analysis.
- `dslc/src/diag.rs` error reporting types and rendering.
- `crates/darcy-runtime` runtime interop for MNIST + EDN parsing.
