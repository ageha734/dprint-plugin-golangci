# dprint-plugin-golangci

A [dprint](https://dprint.dev/) process plugin that integrates [golangci-lint](https://golangci-lint.run/) for Go static analysis and auto-fixing.

## Features

- Integrates golangci-lint into dprint's formatting pipeline
- Auto-fixes issues via `--fix` (configurable)
- Reports unfixable issues as dprint errors
- Unified `dprint fmt` / `dprint check` for Go formatting and linting
- Uses `dprint-core` process plugin protocol (async)

## Installation

Add to your `.dprint.json`:

```json
{
  "golangci": {},
  "plugins": [
    "https://github.com/ageha734/dprint-plugin-golangci/releases/download/v0.1.0/plugin.json@<sha256>"
  ]
}
```

## Prerequisites

- `golangci-lint` must be installed and available in PATH
- A `.golangci.yml` configuration file in your project (optional but recommended)

## Configuration

The `golangci` section in `.dprint.json` supports:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `configPath` | string | (auto-detect) | Path to `.golangci.yml` |
| `fix` | boolean | `true` | Auto-fix issues when possible |

Example:

```json
{
  "golangci": {
    "configPath": ".golangci.yml",
    "fix": true
  }
}
```

## How It Works

1. dprint passes each `.go` file to this plugin
2. The plugin runs `golangci-lint run --fix` on the file
3. If the file was modified (auto-fixed), the new content is returned
4. If unfixable issues remain, an error is reported
5. If no issues, "no change" is returned

## Development

```bash
# Build
cargo build --release

# Test
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

## Release

Tag a version to trigger the release workflow:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The CI builds for 5 platforms and publishes `plugin.json` + zip archives to GitHub Releases.

## Architecture

```text
src/
├── main.rs            # Entry point (dprint-core process plugin bootstrap)
├── lib.rs             # Public exports
├── handler.rs         # AsyncPluginHandler implementation
└── configuration.rs   # Config parsing + CLI arg generation + tests
```

Built on:
- `dprint-core` (process feature) for the stdio message protocol
- `tokio` for async subprocess management
- `anyhow` for error handling

## License

MIT
