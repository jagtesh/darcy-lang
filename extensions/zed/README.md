# Darcy Zed Setup

Darcy LSP is configured through workspace settings in `.zed/settings.json`.

## Install language server

```bash
cargo install --path crates/darcy-lsp
```

## Workspace settings

This repository already includes:

```json
{
  "language_servers": {
    "darcy-lsp": {
      "command": "darcy-lsp",
      "args": []
    }
  },
  "languages": [
    {
      "name": "Darcy",
      "language_servers": ["darcy-lsp"]
    }
  ]
}
```

If your `darcy-lsp` binary is not in `PATH`, set an absolute command path in `.zed/settings.json`.
