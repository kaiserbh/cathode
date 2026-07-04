#!/usr/bin/env bash
# Cathode: build a self-contained, distributable macOS .app (+ .dmg).
#
# Usage:
#   scripts/package-macos.sh [extra `cargo tauri build` args]
#   scripts/package-macos.sh --target aarch64-apple-darwin   # what CI runs
#
# Why this exists: on macOS the binary links `-lmpv` and dyld resolves
# libmpv.2.dylib from its ABSOLUTE Homebrew install name, so a plain
# `cargo tauri build` produces an .app that only runs on machines that have
# `brew install mpv`. This script copies libmpv and its entire transitive dylib
# closure into Cathode.app/Contents/Frameworks, rewrites every install name to
# `@executable_path/../Frameworks/…` (via dylibbundler), re-signs ad-hoc (Apple
# Silicon refuses to run a binary whose signature install_name_tool invalidated),
# and packages the fixed app into a .dmg. The result needs no Homebrew mpv at
# runtime.
#
# Build-host requirements (NOT the end user's machine):
#   - brew install mpv           the libmpv the app is linked/bundled against
#   - brew install dylibbundler  copies + rewrites the dylib closure
#   - dx (Dioxus CLI) + `cargo tauri`, as for any Cathode build
#   - create-dmg is optional; falls back to hdiutil if it is absent or fails.
set -euo pipefail

log()  { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = "Darwin" ] || die "macOS only."
command -v dylibbundler >/dev/null 2>&1 \
  || die "dylibbundler not found. Install it with: brew install dylibbundler"

# Package the fixed app into a plain .dmg with a drag-to-Applications link. Used
# both as the create-dmg fallback and when create-dmg isn't installed; hdiutil is
# part of macOS and needs no Finder/AppleScript, so it never hangs headless CI.
hdiutil_dmg() {
  local stage; stage=$(mktemp -d)
  cp -R "$APP" "$stage/"
  ln -s /Applications "$stage/Applications"
  hdiutil create -volname "Cathode" -srcfolder "$stage" -ov -format UDZO "$DMG" >/dev/null
  rm -rf "$stage"
}

# The install-name prefix dylibbundler bakes into every rewritten reference, and
# the LC_RPATH it adds to each rewritten binary. One constant so the two uses of
# it below can't drift apart.
FW_RPATH="@executable_path/../Frameworks/"

# dylibbundler adds FW_RPATH as an LC_RPATH to each library it rewrites, and can
# add it more than once; modern dyld treats a DUPLICATE LC_RPATH as a fatal load
# error ("Library not loaded … duplicate LC_RPATH"). The bundled deps all
# reference each other by full FW_RPATH… paths (no @rpath), so the rpath is only a
# safety net — reduce each binary to at most one.
dedup_rpath() {
  local f="$1" n
  n=$(otool -l "$f" | grep -cF "path $FW_RPATH (offset") || true
  while [ "${n:-0}" -gt 1 ]; do
    install_name_tool -delete_rpath "$FW_RPATH" "$f"
    n=$((n - 1))
  done
}

# 1. Build only the .app. The .dmg is created later from the fixed app; letting
#    Tauri build the .dmg here would package the un-fixed (Homebrew-dependent) app.
log "Building the macOS .app (cargo tauri build --bundles app)..."
cargo tauri build --bundles app "$@"

# 2. Locate the freshly built bundle. Tauri writes it under the workspace target
#    dir; the triple segment is present only when --target was passed.
APP=$(find target src-tauri/target -type d -path '*release/bundle/macos/Cathode.app' 2>/dev/null | head -n1) || true
[ -n "$APP" ] && [ -d "$APP" ] || die "could not find a built Cathode.app under target/."
APP="$(cd "$(dirname "$APP")" && pwd)/$(basename "$APP")"   # absolutise
PLIST="$APP/Contents/Info.plist"
EXE=$(/usr/libexec/PlistBuddy -c "Print CFBundleExecutable" "$PLIST")
VERSION=$(/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" "$PLIST")
log "App: $APP  (executable: $EXE, version: $VERSION)"

# 3. Copy libmpv + its whole transitive dylib closure into Contents/Frameworks and
#    rewrite ids/deps/exe-refs to @executable_path/../Frameworks/. dylibbundler
#    follows the executable's dependencies, skips system libs (/usr/lib, /System),
#    and copies the rest (all under /opt/homebrew for the libmpv closure). Those
#    are absolute paths it resolves directly, so it runs without prompting; `set
#    -e` aborts on failure and the closure check in step 4 is the real gate on
#    completeness.
log "Bundling the libmpv dylib closure with dylibbundler..."
dylibbundler -of -cd -b \
  -x "$APP/Contents/MacOS/$EXE" \
  -d "$APP/Contents/Frameworks/" \
  -p "$FW_RPATH"

# 3b. Drop duplicate LC_RPATH entries dylibbundler left behind (see dedup_rpath),
#     or the app aborts at launch on modern dyld.
log "De-duplicating LC_RPATH entries..."
dedup_rpath "$APP/Contents/MacOS/$EXE"
for dylib in "$APP"/Contents/Frameworks/*.dylib; do
  [ -e "$dylib" ] || continue
  dedup_rpath "$dylib"
done

# 4. Gate: nothing in the app may still reference a Homebrew path, or it isn't
#    self-contained and would fail on a machine without `brew install mpv`.
log "Verifying no Homebrew paths remain..."
bad=0
check_refs() {
  if otool -L "$1" | grep -Eq '/opt/homebrew|/usr/local/(opt|lib|Cellar)'; then
    warn "still references Homebrew: $1"
    otool -L "$1" | grep -E '/opt/homebrew|/usr/local' >&2
    bad=1
  fi
}
check_refs "$APP/Contents/MacOS/$EXE"
for dylib in "$APP"/Contents/Frameworks/*.dylib; do
  [ -e "$dylib" ] || continue
  check_refs "$dylib"
done
[ "$bad" -eq 0 ] || die "app still depends on Homebrew libraries (see above)."

# 5. Re-sign ad-hoc, inside-out: install_name_tool invalidated every signature,
#    and Apple Silicon refuses to run an invalidly-signed binary. Sign the nested
#    dylibs first, then seal the bundle. Ad-hoc (-s -) matches today's unsigned
#    build; users still clear the quarantine flag on first launch (see README).
log "Ad-hoc re-signing the bundle..."
for dylib in "$APP"/Contents/Frameworks/*.dylib; do
  [ -e "$dylib" ] || continue
  codesign --force --sign - --timestamp=none "$dylib"
done
codesign --force --sign - --timestamp=none "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

# 6. Package the fixed app into a .dmg (with a drag-to-Applications link), named
#    to match Tauri's convention (Cathode_<version>_<arch>.dmg).
ARCH_SUFFIX="aarch64"
case "$(uname -m)" in x86_64) ARCH_SUFFIX="x64";; esac
DMG_DIR="$(dirname "$(dirname "$APP")")/dmg"
DMG="$DMG_DIR/Cathode_${VERSION}_${ARCH_SUFFIX}.dmg"
mkdir -p "$DMG_DIR"
rm -f "$DMG"
log "Creating $DMG ..."
if command -v create-dmg >/dev/null 2>&1; then
  stage=$(mktemp -d)
  cp -R "$APP" "$stage/"
  if create-dmg \
       --volname "Cathode" \
       --window-size 540 380 \
       --icon-size 128 \
       --icon "Cathode.app" 140 190 \
       --app-drop-link 400 190 \
       "$DMG" "$stage" 2>/dev/null; then
    rm -rf "$stage"
  else
    warn "create-dmg failed (headless Finder?); falling back to hdiutil."
    rm -rf "$stage"
    rm -f "$DMG"
    hdiutil_dmg
  fi
else
  hdiutil_dmg
fi

log "Done. Self-contained macOS bundle:"
printf '    App: %s\n' "$APP"
printf '    DMG: %s\n' "$DMG"
