#!/usr/bin/env bash
#
# essh.sh — launcher for the Enhanced SSH Client
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BINARY="$SCRIPT_DIR/target/release/essh"

# Build if binary doesn't exist or source is newer
needs_build() {
    [[ ! -f "$BINARY" ]] && return 0
    local newest_src
    newest_src=$(find "$SCRIPT_DIR/src" -name '*.rs' -newer "$BINARY" 2>/dev/null | head -1)
    [[ -n "$newest_src" ]]
}

if needs_build; then
    echo "Building essh..." >&2
    cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml" 2>&1 \
        | grep -E "Compiling essh|Finished|error" >&2
fi

exec "$BINARY" "$@"
