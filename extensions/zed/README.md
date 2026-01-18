# Darcy Zed Extension

This extension provides syntax highlighting via tree-sitter. To enable the Darcy LSP, add a Zed settings entry that points to the `darcy-lsp` binary.

Example `~/.config/zed/settings.json` (or project `.zed/settings.json`):

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

Install the server binary (from the repo root):

```bash
cargo build -p darcy-lsp
```

Then either add `target/debug` to your `PATH` or install the binary somewhere on your `PATH`.
