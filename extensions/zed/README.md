# Darcy Zed Extension

This folder is a Zed dev extension (it includes `extension.toml` and `languages/darcy/config.toml`).

Install it in Zed via:
- `zed: extensions install dev extension`
- Select: `/Volumes/Dev/code/jagtesh/darcy-lang/extensions/zed`

After install, `.dsl` files map to language `Darcy`.
The current grammar backend is `tree-sitter-scheme`, so extension config uses grammar id `scheme`.
Syntax highlighting rules are in `languages/darcy/highlights.scm`.

Darcy LSP is configured through workspace settings in `.zed/settings.json`.

## Install language server

```bash
cargo install --path crates/darcy-lsp
```

## Workspace settings

This repository already includes:

```json
{
  "lsp": {
    "darcy-lsp": {
      "binary": {
        "path": "darcy-lsp",
        "arguments": []
      }
    }
  },
  "languages": {
    "Darcy": {
      "language_servers": ["darcy-lsp"],
      "formatter": "language_server",
      "format_on_save": "on"
    }
  }
}
```

If your `darcy-lsp` binary is not in `PATH`, set an absolute command path in `.zed/settings.json`.
