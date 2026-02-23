const vscode = require("vscode");
const path = require("path");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function resolveServerCommand() {
  const cfg = vscode.workspace.getConfiguration("darcy.languageServer");
  const command = cfg.get("path", "darcy-lsp");
  const args = cfg.get("args", []);
  return { command, args };
}

function createClient(context) {
  const { command, args } = resolveServerCommand();

  const serverOptions = {
    run: { command, args, transport: TransportKind.stdio },
    debug: { command, args, transport: TransportKind.stdio }
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "darcy" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.dsl")
    },
    outputChannel: vscode.window.createOutputChannel("Darcy LSP")
  };

  return new LanguageClient("darcy-lsp", "Darcy Language Server", serverOptions, clientOptions);
}

function activate(context) {
  client = createClient(context);
  context.subscriptions.push(client.start());

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration("darcy.languageServer")) {
        return;
      }
      if (!client) {
        return;
      }
      client.stop().then(() => {
        client = createClient(context);
        context.subscriptions.push(client.start());
      });
    })
  );
}

function deactivate() {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

module.exports = {
  activate,
  deactivate
};
