# renpyfmt VS Code Extension

This extension launches `renpyfmt lsp` for `.rpy` files.

## Local setup

1. Build the server from the repo root:

```bash
rtk cargo build
```

2. Install extension dependencies:

```bash
cd vscode-extension
npm install
```

3. Compile the extension:

```bash
npm run compile
```

4. Open `vscode-extension` in VS Code and press `F5`.

This starts an Extension Development Host. Open an `.rpy` file there to activate the language client.

## Server path

By default the extension looks for the server at:

```text
../target/debug/renpyfmt
```

relative to the extension directory.

You can override this with the VS Code setting:

```json
"renpyfmt.serverPath": "/absolute/path/to/renpyfmt"
```
