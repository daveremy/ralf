#!/usr/bin/env bash
# ralf uninstaller

set -euo pipefail

# Defaults
PREFIX="${PREFIX:-$HOME/.local}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() { echo -e "${GREEN}[ralf]${NC} $*"; }
warn() { echo -e "${YELLOW}[ralf]${NC} $*" >&2; }
error() { echo -e "${RED}[ralf]${NC} $*" >&2; exit 1; }

usage() {
    cat <<EOF
ralf uninstaller

USAGE:
    uninstall.sh [OPTIONS]

OPTIONS:
    --prefix <DIR>    Installation prefix (default: ~/.local)
    -h, --help        Show this help message

EXAMPLES:
    # Uninstall from default location
    ./uninstall.sh

    # Uninstall from custom prefix
    ./uninstall.sh --prefix /opt/local
EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

main() {
    local binary="${PREFIX}/bin/ralf"

    if [[ ! -f "$binary" ]]; then
        warn "ralf not found at ${binary}"
        exit 0
    fi

    log "Removing ${binary}..."
    rm -f "$binary"
    log "ralf has been uninstalled"
}

main
