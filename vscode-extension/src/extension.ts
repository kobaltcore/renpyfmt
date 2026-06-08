import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  RevealOutputChannelOn,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(
  context: vscode.ExtensionContext,
): Promise<void> {
  const command = getServerCommand(context);
  const repoRoot = path.resolve(context.extensionPath, "..");
  const outputChannel = vscode.window.createOutputChannel("renpyfmt");

  if (!fs.existsSync(command)) {
    void vscode.window.showErrorMessage(
      `renpyfmt executable not found at ${command}. Build the server with \`cargo build\` or set renpyfmt.serverPath.`,
    );
    return;
  }

  const serverOptions: ServerOptions = {
    command,
    args: ["lsp"],
    options: {
      cwd: repoRoot,
      env: {
        ...process.env,
        RUST_BACKTRACE: "1",
      },
    },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "renpy" }],
    outputChannel,
    revealOutputChannelOn: RevealOutputChannelOn.Error,
  };

  client = new LanguageClient(
    "renpyfmt",
    "renpyfmt",
    serverOptions,
    clientOptions,
  );

  context.subscriptions.push(outputChannel, client);
  void client.start().catch((error: unknown) => {
    const message =
      error instanceof Error ? error.message : "Unknown renpyfmt startup failure";
    outputChannel.appendLine(`renpyfmt failed to start: ${message}`);
    void vscode.window.showErrorMessage(`renpyfmt failed to start: ${message}`);
  });
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

function getServerCommand(context: vscode.ExtensionContext): string {
  const config = vscode.workspace.getConfiguration("renpyfmt");
  const configuredPath = config.get<string>("serverPath")?.trim();

  if (configuredPath) {
    return configuredPath;
  }

  return path.resolve(
    context.extensionPath,
    "..",
    "target",
    "debug",
    "renpyfmt",
  );
}
