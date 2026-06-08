# Changelog

## v0.2.0

### Added

- top-level `check` command alongside `format`
- shared multi-input handling for `format` and `check`, supporting one or more files and directories
- stdin formatting and checking via `-` with required `--stdin-filename <PATH>`

### Changed

- Updated Ruff config discovery rules:
  - stdin resolves from `--stdin-filename`'s parent, or the current directory when it has no parent
  - a single file resolves from that file's parent
  - a single directory resolves from that directory
  - multiple path inputs resolve from the current directory

### Documentation

- Updated README usage examples for `renpyfmt format [PATHS]...` and `renpyfmt check [PATHS]...`
- stdin requirements and `check` exit codes
