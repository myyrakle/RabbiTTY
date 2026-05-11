#!/bin/sh
# Rabbitty installer for Linux and macOS.
#   curl -fsSL https://raw.githubusercontent.com/wHoIsDReAmer/RabbiTTY/main/install.sh | sh

set -e

REPO="wHoIsDReAmer/RabbiTTY"
BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/Applications"

err() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)
    case "${os}_${arch}" in
        Linux_x86_64)        echo "linux-amd64" ;;
        Linux_aarch64|Linux_arm64) echo "linux-arm64" ;;
        Darwin_arm64)        echo "macos-arm64" ;;
        *) echo "" ;;
    esac
}

latest_tag() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | sed -nE 's/.*"tag_name": *"([^"]+)".*/\1/p' \
        | head -n1
}

target=$(detect_target)
[ -n "$target" ] || err "unsupported OS/arch: $(uname -s) $(uname -m). Supported: Linux x86_64/aarch64, macOS arm64."

tag=$(latest_tag)
[ -n "$tag" ] || err "failed to resolve latest release tag from GitHub."

case "$target" in
    macos-*) ext="zip" ;;
    *)       ext="tar.gz" ;;
esac

asset="rabbitty-${tag}-${target}.${ext}"
url="https://github.com/${REPO}/releases/download/${tag}/${asset}"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

printf 'Downloading %s...\n' "$asset"
curl -fsSL -o "${tmp}/${asset}" "$url" || err "download failed: $url"

printf 'Extracting...\n'
case "$ext" in
    tar.gz) tar -xzf "${tmp}/${asset}" -C "$tmp" ;;
    zip)    unzip -q "${tmp}/${asset}" -d "$tmp" ;;
esac

case "$target" in
    macos-*)
        app_src=$(find "$tmp" -name 'Rabbitty.app' -type d -maxdepth 4 | head -n1)
        [ -n "$app_src" ] || err "Rabbitty.app not found in archive."
        mkdir -p "$APP_DIR"
        rm -rf "${APP_DIR}/Rabbitty.app"
        cp -R "$app_src" "$APP_DIR/"
        xattr -dr com.apple.quarantine "${APP_DIR}/Rabbitty.app" 2>/dev/null || true
        mkdir -p "$BIN_DIR"
        ln -sf "${APP_DIR}/Rabbitty.app/Contents/MacOS/rabbitty" "${BIN_DIR}/rabbitty"
        printf '\nInstalled Rabbitty.app to %s\n' "$APP_DIR"
        printf 'CLI symlink at %s/rabbitty\n' "$BIN_DIR"
        ;;
    linux-*)
        bin_src=$(find "$tmp" -name 'rabbitty' -type f -maxdepth 4 | head -n1)
        [ -n "$bin_src" ] || err "rabbitty binary not found in archive."
        mkdir -p "$BIN_DIR"
        install -m 0755 "$bin_src" "${BIN_DIR}/rabbitty"
        printf '\nInstalled rabbitty to %s/rabbitty\n' "$BIN_DIR"
        ;;
esac

case ":${PATH}:" in
    *:"${BIN_DIR}":*) ;;
    *)
        printf '\nWarning: %s is not in your PATH.\n' "$BIN_DIR"
        printf 'Add this to your shell profile (~/.bashrc, ~/.zshrc, ~/.profile):\n'
        printf '  export PATH="$HOME/.local/bin:$PATH"\n'
        ;;
esac

printf "\nDone. Run 'rabbitty' to start.\n"
