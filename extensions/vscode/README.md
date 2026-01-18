# Darcy VS Code Extension

This extension provides Darcy syntax highlighting and hooks VS Code up to the Darcy language server.

## Requirements

- Node.js (for installing extension dependencies)
- The Darcy language server binary: `darcy-lsp`

## Install (dev)

1. Install extension dependencies:

```sh
cd extensions/vscode
npm install
```

2. Ensure the language server is available on your PATH:

```sh
cargo install --path crates/darcy-lsp
```

Alternatively, set `DARCY_LSP_PATH` to a specific `darcy-lsp` binary.

3. In VS Code, run `Developer: Install Extension from Location...` and select `extensions/vscode`.

## Notes

- The extension starts `darcy-lsp` automatically when you open a `.dsl` file.
- If you keep a local release build, the extension will also look for
  `target/release/darcy-lsp` relative to the repository root.

## Packaging

```sh
cd extensions/vscode
npx @vscode/vsce package
```
