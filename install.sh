#!/usr/bin/env bash
set -euo pipefail

# Rick — Multi-agent AI Orchestration CLI
# One-line installer: curl -fsSL https://raw.githubusercontent.com/OWNER/rick/main/install.sh | bash
#
# Options:
#   -u, --universe <url>   Clone and compile a Universe after install
#   --install-deps          Auto-install MCP servers required by agents
#   -h, --help              Show help

# ─── Configuration ────────────────────────────────────────────────────────────

RICK_REPO="Sagi363/Rick-POC"
RICK_VERSION="latest"
RICK_BIN_NAME="rick"

# ─── Colors ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
WHITE='\033[0;97m'
DIM='\033[0;90m'
RESET='\033[0m'

# ─── Helpers ──────────────────────────────────────────────────────────────────

info()  { printf "${CYAN}%s${RESET}\n" "$1"; }
ok()    { printf "  ${GREEN}✓${RESET} %s\n" "$1"; }
warn()  { printf "  ${YELLOW}!${RESET} %s\n" "$1"; }
fail()  { printf "  ${RED}✗${RESET} %s\n" "$1"; }
step()  { printf "\n${WHITE}%s${RESET}\n" "$1"; }

# ─── Parse Arguments ──────────────────────────────────────────────────────────

UNIVERSE_URL=""
INSTALL_DEPS=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -u|--universe)
            UNIVERSE_URL="$2"
            shift 2
            ;;
        --install-deps)
            INSTALL_DEPS="--install-deps"
            shift
            ;;
        -h|--help)
            echo "Rick Installer"
            echo ""
            echo "Usage: install.sh [options]"
            echo ""
            echo "Options:"
            echo "  -u, --universe <url>   Clone and compile a Universe after install"
            echo "  --install-deps          Auto-install MCP servers required by agents"
            echo "  -h, --help              Show help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# ─── Banner ───────────────────────────────────────────────────────────────────

printf "\n"
printf "${CYAN}  Rick — Multi-agent AI Orchestration CLI${RESET}\n"
printf "${DIM}  Installing...${RESET}\n"
printf "\n"

# ─── Stage 1: Prerequisites ──────────────────────────────────────────────────

step "Checking prerequisites..."

# Git (required)
if command -v git &>/dev/null; then
    ok "git $(git --version | awk '{print $3}')"
else
    fail "git not found — install git first: https://git-scm.com"
    exit 1
fi

# Claude Code (warn if missing, don't block)
if command -v claude &>/dev/null; then
    ok "Claude Code found"
else
    warn "Claude Code not detected"
    printf "      ${DIM}Rick CLI will install, but /Rick skill won't work until you install Claude Code${RESET}\n"
    printf "      ${DIM}Install: npm install -g @anthropic-ai/claude-code${RESET}\n"
fi

# Rust/cargo (needed only if we can't download a binary)
HAS_CARGO=false
if command -v cargo &>/dev/null; then
    HAS_CARGO=true
elif [[ -f "$HOME/.cargo/env" ]]; then
    source "$HOME/.cargo/env"
    if command -v cargo &>/dev/null; then
        HAS_CARGO=true
    fi
fi

# ─── Stage 2: Detect Platform ────────────────────────────────────────────────

step "Detecting platform..."

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
    darwin) OS="darwin" ;;
    linux)  OS="linux" ;;
    *)
        fail "Unsupported OS: $OS"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)  ARCH="amd64" ;;
    arm64|aarch64) ARCH="arm64" ;;
    *)
        fail "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

ok "Platform: ${OS}-${ARCH}"

# ─── Stage 3: Download or Build Binary ───────────────────────────────────────

step "Installing Rick binary..."

BINARY_DOWNLOADED=false

# Try downloading pre-compiled binary from GitHub Releases
if [[ "$RICK_VERSION" == "latest" ]]; then
    DOWNLOAD_URL="https://github.com/${RICK_REPO}/releases/latest/download/rick-${OS}-${ARCH}"
else
    DOWNLOAD_URL="https://github.com/${RICK_REPO}/releases/download/${RICK_VERSION}/rick-${OS}-${ARCH}"
fi

if command -v curl &>/dev/null; then
    TMP_BIN="$(mktemp)"
    if curl -fsSL "$DOWNLOAD_URL" -o "$TMP_BIN" 2>/dev/null; then
        chmod +x "$TMP_BIN"
        # Verify it's a real binary (not an HTML error page)
        if "$TMP_BIN" --version &>/dev/null; then
            BINARY_DOWNLOADED=true
            ok "Downloaded rick from GitHub Releases"
        else
            rm -f "$TMP_BIN"
        fi
    else
        rm -f "$TMP_BIN"
    fi
fi

# Fallback: build from source if download failed
if [[ "$BINARY_DOWNLOADED" != "true" ]]; then
    if [[ "$HAS_CARGO" == "true" ]]; then
        warn "Pre-compiled binary not available, building from source..."

        # Check if we're in the Rick repo (has cli/Cargo.toml)
        if [[ -f "cli/Cargo.toml" ]]; then
            (cd cli && cargo build --release 2>&1 | grep -v "^warning:")
            TMP_BIN="cli/target/release/rick"
            if [[ -f "$TMP_BIN" ]]; then
                BINARY_DOWNLOADED=true
                ok "Built rick from source"
            else
                fail "Build failed"
                exit 1
            fi
        else
            # Clone the repo, build, clean up
            RICK_TMP_DIR="$(mktemp -d)"
            info "  Cloning Rick repo..."
            git clone --depth 1 "https://github.com/${RICK_REPO}.git" "$RICK_TMP_DIR" 2>/dev/null || {
                fail "Could not clone Rick repo and no pre-compiled binary available"
                rm -rf "$RICK_TMP_DIR"
                exit 1
            }
            (cd "$RICK_TMP_DIR/cli" && cargo build --release 2>&1 | grep -v "^warning:")
            TMP_BIN="$RICK_TMP_DIR/cli/target/release/rick"
            if [[ -f "$TMP_BIN" ]]; then
                BINARY_DOWNLOADED=true
                ok "Built rick from source"
            else
                fail "Build failed"
                rm -rf "$RICK_TMP_DIR"
                exit 1
            fi
        fi
    else
        fail "No pre-compiled binary available and cargo not found"
        printf "      ${DIM}Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${RESET}\n"
        exit 1
    fi
fi

# ─── Stage 4: Install Binary to PATH ─────────────────────────────────────────

step "Installing to PATH..."

INSTALL_DIR=""

# Try /usr/local/bin first (needs sudo)
if [[ -d "/usr/local/bin" ]] && [[ -w "/usr/local/bin" ]]; then
    # Writable without sudo
    cp "$TMP_BIN" "/usr/local/bin/$RICK_BIN_NAME"
    chmod +x "/usr/local/bin/$RICK_BIN_NAME"
    INSTALL_DIR="/usr/local/bin"
    ok "Installed to /usr/local/bin/rick"
elif command -v sudo &>/dev/null; then
    # Try with sudo
    printf "  ${DIM}Installing to /usr/local/bin (requires sudo)...${RESET}\n"
    if sudo cp "$TMP_BIN" "/usr/local/bin/$RICK_BIN_NAME" 2>/dev/null && sudo chmod +x "/usr/local/bin/$RICK_BIN_NAME" 2>/dev/null; then
        INSTALL_DIR="/usr/local/bin"
        ok "Installed to /usr/local/bin/rick"
    fi
fi

# Fallback to ~/.local/bin
if [[ -z "$INSTALL_DIR" ]]; then
    warn "/usr/local/bin not available, using ~/.local/bin"
    mkdir -p "$HOME/.local/bin"
    cp "$TMP_BIN" "$HOME/.local/bin/$RICK_BIN_NAME"
    chmod +x "$HOME/.local/bin/$RICK_BIN_NAME"
    INSTALL_DIR="$HOME/.local/bin"
    ok "Installed to ~/.local/bin/rick"

    # Check if ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        printf "\n"
        warn "~/.local/bin is not in your PATH"
        printf "      ${WHITE}Add this to your shell profile (~/.zshrc or ~/.bashrc):${RESET}\n"
        printf "\n"
        printf "      ${CYAN}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}\n"
        printf "\n"
        printf "      ${DIM}Then restart your terminal or run: source ~/.zshrc${RESET}\n"
    fi
fi

# Clean up temp files (but not if we built in-tree)
if [[ -n "${RICK_TMP_DIR:-}" ]]; then
    rm -rf "$RICK_TMP_DIR"
fi

# Verify installation
RICK_PATH="${INSTALL_DIR}/${RICK_BIN_NAME}"
if "$RICK_PATH" --version &>/dev/null; then
    RICK_VER=$("$RICK_PATH" --version)
    ok "Verified: $RICK_VER"
else
    fail "Installation verification failed"
    exit 1
fi

# ─── Stage 5: Run rick setup ─────────────────────────────────────────────────

step "Running Rick setup..."

SETUP_ARGS=""
if [[ -n "$UNIVERSE_URL" ]]; then
    SETUP_ARGS="--universe $UNIVERSE_URL"
fi
if [[ -n "$INSTALL_DEPS" ]]; then
    SETUP_ARGS="$SETUP_ARGS $INSTALL_DEPS"
fi

# Run setup — redirect stdin from /dev/tty so interactive prompts work
# even when install.sh is piped (curl ... | bash)
if [ -e /dev/tty ]; then
    "$RICK_PATH" setup $SETUP_ARGS < /dev/tty
else
    # No TTY (Docker, CI) — fall back to non-interactive
    "$RICK_PATH" setup $SETUP_ARGS --non-interactive
fi

# ─── Done ─────────────────────────────────────────────────────────────────────

printf "\n"
printf "${GREEN}  Rick is ready!${RESET}\n"
printf "\n"
printf "  ${WHITE}Binary:${RESET}  ${RICK_PATH}\n"
printf "  ${WHITE}Skill:${RESET}   ~/.claude/skills/rick/ + Rick/\n"
printf "  ${WHITE}Persona:${RESET} ~/.rick/persona/\n"
if [[ -n "$UNIVERSE_URL" ]]; then
    printf "  ${WHITE}Universe:${RESET} cloned and compiled\n"
fi
printf "\n"
printf "  ${DIM}Open Claude Code and type /Rick to get started${RESET}\n"
printf "\n"
