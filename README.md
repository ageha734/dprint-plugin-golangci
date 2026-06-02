# dprint-plugin-golangci

A [dprint](https://dprint.dev/) process plugin that runs [golangci-lint](https://golangci-lint.run/) for Go static analysis and auto-fixing.

## Features

- Integrates golangci-lint into dprint's formatting pipeline
- Auto-fixes issues that golangci-lint can fix (`--fix`)
- Reports unfixable issues as dprint errors
- Enables unified `dprint fmt` / `dprint check` for both formatting and linting

## Installation

Add to your `.dprint.json`:

```json
{
  "golangci": {},
  "plugins": [
    "https://github.com/ageha734/dprint-plugin-golangci/releases/download/v0.1.0/dprint-plugin-golangci"
  ]
}
```

## Prerequisites

- `golangci-lint` must be installed and available in PATH

## Configuration

The `golangci` section in `.dprint.json` currently uses default settings. golangci-lint configuration is read from the standard `.golangci.yml` file in your project.

## How It Works

1. dprint passes each `.go` file to this plugin
2. The plugin runs `golangci-lint run --fix` on the file
3. If the file was modified (auto-fixed), the new content is returned
4. If unfixable issues remain, an error is reported
5. If no issues, "no change" is returned

## Development

```bash
cargo build --release
```

## License

MIT
