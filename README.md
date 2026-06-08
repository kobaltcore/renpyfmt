![](docs/banner.jpg "renpyfmt logo")

# renpyfmt

`renpyfmt` is an opinionated source code formatter for Ren'Py script.

It ships with a complete, standalone parser for the Ren'Py language, allowing for deep understanding of the code and thus proper formatting. Embedded Python blocks and Python files are formatted via `ruff`.

## Supported Platforms

| OS      | amd64 | aarch64 |
| ------- | ----- | ------- |
| Linux   | ✅    | ✅      |
| macOS   | ✅    | ✅      |
| Windows | ✅    | ❌      |

## Usage

```sh
renpyfmt format [PATHS]...
renpyfmt check [PATHS]...
renpyfmt format - --stdin-filename script.rpy
renpyfmt check - --stdin-filename script.rpy
```

`format` rewrites files in place, or writes formatted output to stdout when reading from stdin.

`check` performs the same discovery and formatting pass without writing changes. It exits with:

- `0` when every input is already formatted
- `1` when one or more inputs would change
- `2` on real errors

Stdin input always requires `--stdin-filename <PATH>` so `renpyfmt` can choose `.rpy` vs `.py` formatting and discover Ruff configuration from the right directory.

## Formatting in VS Code

To format `.rpy` files from VS Code, install:

- a Ren'Py language extension for `.rpy` files
- [Advanced Local Formatters](https://marketplace.visualstudio.com/items?itemName=webfreak.advanced-local-formatters)

Then add this to `.vscode/settings.json`:

```json
{
  "advancedLocalFormatters.formatters": [
    {
      "command": ["renpyfmt", "format", "-", "--stdin-filename", "$absoluteFilePath"],
      "languages": ["renpy"]
    }
  ],
  "[renpy]": {
    "editor.defaultFormatter": "webfreak.advanced-local-formatters",
    "editor.formatOnSave": true
  }
}
```

If your Ren'Py extension uses a different language id than `renpy`, replace it in both places above. If `renpyfmt` is not on your `PATH`, use an absolute path to the executable instead of `renpyfmt`.

## Installation

### Automatic

```sh
brew install kobaltcore/renpyfmt/renpyfmt
```

`renpyfmt` comes with several installation options for the various supported platforms. Please check out the available options for the [latest release](https://github.com/kobaltcore/renpyfmt/releases/latest).

### Manual (Linux / Windows / macOS)

Download the pre-built binaries for your operating system and architecture for the [latest release](https://github.com/kobaltcore/renpyfmt/releases/latest) and extract the resulting tar file.

After this, either add the binaries to your PATH or use them directly from within the download directory.

<a href="https://unsplash.com/photos/E8Ufcyxz514?utm_source=unsplash&utm_medium=referral&utm_content=creditShareLink">Photo by Milad Fakurian on Unsplash</a>

```

```
