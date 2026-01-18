const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function isExecutable(filePath) {
  try {
    const stat = fs.statSync(filePath);
    return stat.isFile();
  } catch {
    return false;
  }
}

function resolveServerCommand(context) {
  const candidates = [
    process.env.DARCY_LSP_PATH,
    path.join(
      context.extensionPath,
      "..",
      "..",
      "crates",
      "darcy-lsp",
      "target",
      "release",
      "darcy-lsp"
    ),
    path.join(context.extensionPath, "..", "..", "target", "release", "darcy-lsp"),
    "darcy-lsp",
  ].filter(Boolean);

  for (const candidate of candidates) {
    if (candidate === "darcy-lsp") {
      return candidate;
    }
    if (isExecutable(candidate)) {
      return candidate;
    }
  }

  return "darcy-lsp";
}

function activate(context) {
  const command = resolveServerCommand(context);
  const serverOptions = {
    command,
    args: [],
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "darcy" }],
    outputChannelName: "Darcy LSP",
  };

  client = new LanguageClient("darcy-lsp", "Darcy LSP", serverOptions, clientOptions);
  context.subscriptions.push(client.start());
}

function deactivate() {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

module.exports = {
  activate,
  deactivate,
};
