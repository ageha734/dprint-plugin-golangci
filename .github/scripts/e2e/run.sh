#!/usr/bin/env bash
set -euo pipefail

BINARY_PATH="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
MODE="${2:-}"          # "installed" or "auto-install"
VERSION="${3:-}"       # golangci-lint version (required for auto-install)
E2E_DIR="/tmp/e2e-dprint-golangci"

sha256() { sha256sum "$1" 2>/dev/null || shasum -a 256 "$1"; }

# golangci-lint version -> minimum Go version
declare -A COMPAT_MAP=(
  ["2.5.0"]="1.24"
  ["2.4.0"]="1.24"
  ["2.3.0"]="1.23"
  ["2.2.0"]="1.23"
  ["2.1.0"]="1.23"
  ["2.0.0"]="1.23"
  ["1.64.8"]="1.23"
  ["1.64.0"]="1.23"
  ["1.63.4"]="1.23"
  ["1.62.2"]="1.22"
  ["1.61.0"]="1.22"
  ["1.60.3"]="1.21"
  ["1.59.1"]="1.21"
  ["1.58.2"]="1.21"
  ["1.57.2"]="1.20"
  ["1.56.2"]="1.20"
  ["1.55.2"]="1.20"
)

detect_go_version() {
  if command -v go &>/dev/null; then
    go version | grep -oE 'go[0-9]+\.[0-9]+' | sed 's/go//'
  else
    echo ""
  fi
}

detect_golangci_version() {
  if command -v golangci-lint &>/dev/null; then
    golangci-lint version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1
  else
    echo ""
  fi
}

go_version_for_golangci() {
  local golangci_version="$1"
  # exact match
  if [[ -n "${COMPAT_MAP[$golangci_version]:-}" ]]; then
    echo "${COMPAT_MAP[$golangci_version]}"
    return
  fi
  # fallback: match major.minor
  local major_minor="${golangci_version%.*}"
  for key in "${!COMPAT_MAP[@]}"; do
    if [[ "$key" == "$major_minor"* ]]; then
      echo "${COMPAT_MAP[$key]}"
      return
    fi
  done
  echo "1.22"
}

check_go_compatibility() {
  local go_version="$1"
  local required="$2"

  local go_major go_minor req_major req_minor
  go_major=$(echo "$go_version" | cut -d. -f1)
  go_minor=$(echo "$go_version" | cut -d. -f2)
  req_major=$(echo "$required" | cut -d. -f1)
  req_minor=$(echo "$required" | cut -d. -f2)

  if [ "$go_major" -gt "$req_major" ] || { [ "$go_major" -eq "$req_major" ] && [ "$go_minor" -ge "$req_minor" ]; }; then
    return 0
  fi
  return 1
}

setup_test_project() {
  local dir="$1"
  local go_version="$2"
  local golangci_ver="$3"
  rm -rf "$dir"
  mkdir -p "$dir"

  cat > "$dir/main.go" <<'GO'
package main

import "fmt"

func unused() {}

func main() {
	fmt.Println("hello")
}
GO

  cat > "$dir/go.mod" <<MOD
module e2e

go ${go_version}
MOD

  local major="${golangci_ver%%.*}"
  if [[ "$major" == "2" ]]; then
    cat > "$dir/.golangci.yml" <<'YAML'
version: "2"
linters:
  enable:
    - unused
YAML
  else
    cat > "$dir/.golangci.yml" <<'YAML'
linters:
  enable:
    - unused
YAML
  fi
}

run_test() {
  local mode="$1"        # "installed" or "auto-install:<version>"
  local test_dir="$E2E_DIR/$mode"

  local golangci_version=""
  local dprint_config_version=""

  if [[ "$mode" == "installed" ]]; then
    golangci_version=$(detect_golangci_version)
    if [[ -z "$golangci_version" ]]; then
      echo "=== SKIP (installed mode): golangci-lint not found ==="
      return 0
    fi
    echo "=== Testing with installed golangci-lint v${golangci_version} ==="
  else
    golangci_version="${mode#auto-install:}"
    dprint_config_version="$golangci_version"
    echo "=== Testing auto-install of golangci-lint v${golangci_version} ==="
  fi

  local required_go
  required_go=$(go_version_for_golangci "$golangci_version")
  local current_go
  current_go=$(detect_go_version)

  if [[ -z "$current_go" ]]; then
    echo "✗ Go not found in PATH"
    return 1
  fi

  if ! check_go_compatibility "$current_go" "$required_go"; then
    echo "=== SKIP: Go ${current_go} < required ${required_go} for golangci-lint v${golangci_version} ==="
    return 0
  fi

  setup_test_project "$test_dir" "$current_go" "$golangci_version"
  cd "$test_dir"

  dprint clear-cache 2>/dev/null || true
  zip -j plugin-binary.zip "$BINARY_PATH"
  local zip_checksum
  zip_checksum=$(sha256 plugin-binary.zip | cut -d' ' -f1)

  cat > plugin.json <<JSON
{
  "schemaVersion": 2,
  "kind": "process",
  "name": "dprint-plugin-golangci",
  "version": "0.1.0",
  "linux-x86_64": {
    "reference": "$test_dir/plugin-binary.zip",
    "checksum": "$zip_checksum"
  },
  "linux-aarch64": {
    "reference": "$test_dir/plugin-binary.zip",
    "checksum": "$zip_checksum"
  },
  "darwin-x86_64": {
    "reference": "$test_dir/plugin-binary.zip",
    "checksum": "$zip_checksum"
  },
  "darwin-aarch64": {
    "reference": "$test_dir/plugin-binary.zip",
    "checksum": "$zip_checksum"
  }
}
JSON

  local plugin_checksum
  plugin_checksum=$(sha256 plugin.json | cut -d' ' -f1)

  local version_config=""
  if [[ -n "$dprint_config_version" ]]; then
    version_config="\"version\": \"$dprint_config_version\","
  fi

  cat > dprint.json <<JSON
{
  "golangci": {
    ${version_config}
    "fix": true
  },
  "plugins": [
    "$test_dir/plugin.json@$plugin_checksum"
  ]
}
JSON

  local output
  output=$(dprint check -- main.go 2>&1 || true)
  echo "$output"

  if echo "$output" | grep -q "unused"; then
    echo "✓ ${mode}: passed (go ${current_go})"
    return 0
  else
    echo "✗ ${mode}: expected 'unused' lint error not found"
    return 1
  fi
}

# --- Main ---

if [[ -z "$MODE" ]]; then
  echo "Usage: $0 <binary> <mode> [version]"
  echo "  mode: 'installed' or 'auto-install'"
  echo "  version: golangci-lint version (required for auto-install)"
  exit 1
fi

if [[ "$MODE" == "auto-install" && -z "$VERSION" ]]; then
  echo "Error: version is required for auto-install mode"
  exit 1
fi

echo "Environment:"
echo "  Go: $(detect_go_version || echo 'not found')"
echo "  golangci-lint: $(detect_golangci_version || echo 'not found')"
echo "  dprint: $(dprint --version 2>/dev/null || echo 'not found')"
echo "  Mode: $MODE"
echo "  Version: ${VERSION:-auto-detect}"
echo

if [[ "$MODE" == "installed" ]]; then
  run_test "installed"
else
  run_test "auto-install:$VERSION"
fi
