#!/bin/sh
# Cathode: build-from-source installer for Arch Linux (and Arch-based distros).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/kaiserbh/cathode/main/scripts/install-arch.sh | sh
#
# Optional environment overrides:
#   PREFIX=$HOME/.local   install into a user prefix instead of /usr/local
#   REF=v0.5.1            build a specific tag/branch instead of the latest release tag
#   SKIP_DEPS=1           skip the pacman dependency install (you manage deps yourself)
#
# What it does: installs the system dependencies via pacman, installs the build
# CLIs (dx + tauri-cli) at the versions CI uses, clones the repo at the latest
# release tag, builds a release binary, and installs it plus a .desktop entry and
# icon so Cathode shows up in your application launcher.
#
# This is POSIX sh (works under /bin/sh) so it is safe to pipe from curl.
set -eu

REPO_URL="https://github.com/kaiserbh/cathode.git"
PREFIX="${PREFIX:-/usr/local}"
DX_VERSION="0.7.9"

log() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# Run a command as root when not already root; prefer sudo, fall back to su.
as_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  else
    die "need root to run: $* (install sudo or run this script as root)"
  fi
}

command -v pacman >/dev/null 2>&1 \
  || die "pacman not found; this installer is for Arch-based distros only."

# 1. System dependencies. `mpv` provides libmpv.so.2 (what the libmpv2 crate
#    links); `rust` provides cargo. base-devel pulls in the C toolchain + pkgconf.
if [ "${SKIP_DEPS:-0}" = "1" ]; then
  log "SKIP_DEPS=1 set; assuming system dependencies are already installed."
else
  log "Installing system dependencies via pacman..."
  as_root pacman -S --needed --noconfirm \
    base-devel git rust pkgconf openssl \
    mpv gtk3 webkit2gtk-4.1
fi

# 2. Build CLIs. `dx` (Dioxus CLI) is invoked by tauri's beforeBuildCommand;
#    `tauri-cli` provides `cargo tauri`. Pin dx to the version CI uses. Building
#    these from source is slow the first time, so skip if already present.
ensure_cargo_bin() {
  # ensure_cargo_bin <command> <crate-spec> [extra cargo-install args...]
  cmd="$1"; spec="$2"; shift 2
  if command -v "$cmd" >/dev/null 2>&1; then
    log "$cmd already installed ($(command -v "$cmd")); skipping."
  else
    log "Installing $spec (this can take several minutes)..."
    cargo install --locked "$@" "$spec"
  fi
}

# cargo's bin dir may not be on PATH in a piped-from-curl shell; add it.
case ":$PATH:" in
  *":$HOME/.cargo/bin:"*) ;;
  *) PATH="$HOME/.cargo/bin:$PATH"; export PATH ;;
esac

ensure_cargo_bin dx "dioxus-cli@${DX_VERSION}"
ensure_cargo_bin cargo-tauri "tauri-cli@^2"

# 3. Clone at the latest release tag (or REF if provided).
workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT
log "Cloning $REPO_URL into $workdir..."
git clone --quiet "$REPO_URL" "$workdir/cathode"
cd "$workdir/cathode"

if [ -n "${REF:-}" ]; then
  ref="$REF"
else
  ref="$(git -c 'versionsort.suffix=-' tag --list 'v*' --sort=-v:refname | head -n1)"
  [ -n "$ref" ] || die "no release tag found; set REF=<tag-or-branch> to choose one."
fi
log "Building ref: $ref"
git checkout --quiet "$ref"

# 4. Build the release binary. --no-bundle still runs the frontend
#    beforeBuildCommand (dx bundle) but skips AppImage/.deb packaging, since we
#    install the plain binary and rely on the pacman-installed system libraries.
log "Building Cathode (release)..."
cargo tauri build --no-bundle

binary="src-tauri/target/release/cathode"
[ -f "$binary" ] || binary="target/release/cathode"
[ -f "$binary" ] || die "build did not produce a 'cathode' binary."

# 5. Install binary + desktop entry + icon.
log "Installing into $PREFIX ..."
install_root() { as_root install "$@"; }
# When installing under a user-owned prefix, no root is needed.
case "$PREFIX" in
  "$HOME"/*) install_root() { install "$@"; } ;;
esac

install_root -Dm755 "$binary" "$PREFIX/bin/cathode"
install_root -Dm644 "src-tauri/icons/128x128.png" \
  "$PREFIX/share/icons/hicolor/128x128/apps/cathode.png"

desktop="$workdir/cathode.desktop"
cat > "$desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Cathode
Comment=Cross-platform IPTV player
Exec=$PREFIX/bin/cathode
Icon=cathode
Terminal=false
Categories=AudioVideo;Player;TV;
EOF
install_root -Dm644 "$desktop" "$PREFIX/share/applications/cathode.desktop"

log "Cathode installed: $PREFIX/bin/cathode"
printf '    Uninstall with: rm -f %s/bin/cathode %s/share/applications/cathode.desktop %s/share/icons/hicolor/128x128/apps/cathode.png\n' \
  "$PREFIX" "$PREFIX" "$PREFIX"
