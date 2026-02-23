# Darcy Zed Extension

This folder is a Zed dev extension (it includes `extension.toml` and `languages/darcy/config.toml`).

Install it in Zed via:
- `zed: extensions install dev extension`
- Select: `/Volumes/Dev/code/jagtesh/darcy-lang/extensions/zed`

After install, `.dsl` files map to language `Darcy`.
The current grammar backend is `tree-sitter-scheme`, so extension config uses grammar id `scheme`.
Syntax highlighting rules are in `languages/darcy/highlights.scm`.

Darcy LSP is launched by the extension itself (via `extension.wasm`).

## Install language server binary

```bash
cargo install --path crates/darcy-lsp
```

No special workspace settings are required for LSP startup.
