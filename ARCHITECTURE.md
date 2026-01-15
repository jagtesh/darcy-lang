# Architecture

This compiler is organized as a small set of explicit, end-to-end phases. Each phase has a
single input and output type so changes remain local and easy to review.

Pipeline
- Lexing: `lexer::lex` transforms source text into `Vec<Tok>`.
- Parsing: `parser::Parser` transforms tokens into `Vec<Sexp>`.
- AST: `ast::{parse_toplevel}` transforms sexps into `Vec<Top>`.
- Typecheck: `typecheck::typecheck_tops` builds a `TypecheckedProgram` with typed functions.
- Lowering: `lower::lower_program` converts the pipeline output into Rust source.

Entrypoints
- Library API: `dslc::analyze` and `dslc::compile` in `dslc/src/lib.rs`.
- CLI: `dslc/src/main.rs` reads a file and prints either Rust output or a diagnostic.

Directory map
- `dslc/src/lexer.rs` tokenization and spans.
- `dslc/src/parser.rs` s-expression parsing.
- `dslc/src/ast.rs` AST types and syntax-level parsing helpers.
- `dslc/src/typecheck.rs` type inference and typed program construction.
- `dslc/src/typed.rs` typed AST helpers.
- `dslc/src/lower.rs` Rust codegen.
- `dslc/src/pipeline.rs` glue for end-to-end analysis.
- `dslc/src/diag.rs` error reporting types and rendering.
