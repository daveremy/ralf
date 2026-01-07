#!/usr/bin/env bash
# ralf installer
# Usage: curl -fsSL https://raw.githubusercontent.com/<OWNER>/ralf/main/install/install.sh | bash

set -euo pipefail

# Defaults
VERSION="${VERSION:-latest}"
PREFIX="${PREFIX:-$HOME/.local}"
DRY_RUN=false
EXPECTED_SHA256=""
REPO="dremy/ralf"

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
ralf installer

USAGE:
    install.sh [OPTIONS]

OPTIONS:
    --version <VERSION>    Version to install (default: latest)
    --prefix <DIR>         Installation prefix (default: ~/.local)
    --sha256 <HASH>        Expected SHA256 checksum for verification
    --dry-run              Show what would be done without making changes
    -h, --help             Show this help message

EXAMPLES:
    # Install latest version
    curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install/install.sh | bash

    # Install specific version with checksum
    curl -fsSL https://raw.githubusercontent.com/${REPO}/v0.1.0/install/install.sh | \\
        bash -s -- --version v0.1.0 --sha256 abc123...

    # Install to custom prefix
    curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install/install.sh | \\
        bash -s -- --prefix /opt/local
EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        --sha256)
            EXPECTED_SHA256="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
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

# Detect OS and architecture
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="darwin" ;;
        *)       error "Unsupported OS: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64)  arch="amd64" ;;
        aarch64) arch="arm64" ;;
        arm64)   arch="arm64" ;;
        *)       error "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "${os}-${arch}"
}

# Get latest version from GitHub API
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | \
        grep '"tag_name"' | \
        sed -E 's/.*"([^"]+)".*/\1/'
}

# Download and verify binary
download_binary() {
    local version="$1"
    local platform="$2"
    local dest="$3"

    local artifact="ralf-${platform}.tar.gz"
    local url="https://github.com/${REPO}/releases/download/${version}/${artifact}"

    log "Downloading ${artifact} from ${version}..."

    if [[ "$DRY_RUN" == "true" ]]; then
        log "[DRY RUN] Would download: ${url}"
        return 0
    fi

    local tmpdir
    tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" EXIT

    curl -fsSL -o "${tmpdir}/${artifact}" "$url" || \
        error "Failed to download ${url}"

    # Verify checksum if provided
    if [[ -n "$EXPECTED_SHA256" ]]; then
        local actual_sha256
        if command -v sha256sum &>/dev/null; then
            actual_sha256=$(sha256sum "${tmpdir}/${artifact}" | awk '{print $1}')
        elif command -v shasum &>/dev/null; then
            actual_sha256=$(shasum -a 256 "${tmpdir}/${artifact}" | awk '{print $1}')
        else
            warn "No sha256sum or shasum available, skipping verification"
            actual_sha256="$EXPECTED_SHA256"
        fi

        if [[ "$actual_sha256" != "$EXPECTED_SHA256" ]]; then
            error "Checksum mismatch!\n  Expected: ${EXPECTED_SHA256}\n  Actual:   ${actual_sha256}"
        fi
        log "Checksum verified"
    fi

    # Extract and install binary
    log "Extracting..."
    tar -xzf "${tmpdir}/${artifact}" -C "${tmpdir}"

    mkdir -p "${dest}/bin"
    mv "${tmpdir}/ralf" "${dest}/bin/ralf"
    chmod +x "${dest}/bin/ralf"
}

main() {
    log "ralf installer"

    local platform
    platform=$(detect_platform)
    log "Detected platform: ${platform}"

    # Resolve version
    if [[ "$VERSION" == "latest" ]]; then
        VERSION=$(get_latest_version)
        if [[ -z "$VERSION" ]]; then
            error "Could not determine latest version"
        fi
    fi
    log "Version: ${VERSION}"
    log "Install prefix: ${PREFIX}"

    if [[ "$DRY_RUN" == "true" ]]; then
        log "[DRY RUN] Would install ralf ${VERSION} to ${PREFIX}/bin/ralf"
        exit 0
    fi

    download_binary "$VERSION" "$platform" "$PREFIX"

    log "Installed ralf to ${PREFIX}/bin/ralf"
    log ""
    log "Make sure ${PREFIX}/bin is in your PATH:"
    log "  export PATH=\"${PREFIX}/bin:\$PATH\""
    log ""
    log "Then run: ralf --version"
}

main
