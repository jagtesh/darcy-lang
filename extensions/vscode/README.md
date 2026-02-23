# Darcy VS Code Extension

This extension wires `.dsl` files to the `darcy-lsp` server.

## Local development

1. Install the language server binary:
   - `cargo install --path crates/darcy-lsp`
2. Install extension dependencies:
   - `cd extensions/vscode && npm install`
3. In VS Code, open this folder and run `F5` to launch the Extension Development Host.

## Keep grammar in sync

- Update both VS Code and Zed grammar files from shared defs:
  - `npm run grammar:update`
- Verify no drift:
  - `npm run grammar:check`

## Settings

- `darcy.languageServer.path` (default: `darcy-lsp`)
- `darcy.languageServer.args` (default: `[]`)
