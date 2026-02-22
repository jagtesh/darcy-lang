# Cargo Darcy basic interop test

This test builds a Darcy crate with multiple modules and uses it from a Rust
binary crate. The Rust output must match the Darcy output.

## Run

```bash
cd tests/integration/cargo_darcy_basic/app
cargo run
```

Expected output:

```
darcy_main=90 darcy_calc=90 rust=90
```
