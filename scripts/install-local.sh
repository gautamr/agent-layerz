#!/bin/sh
# Build alz from this checkout as one self-contained file (alz plus its runtime
# dependencies) and install it as a regular executable: no symlink, and no
# node_modules at the destination.
#
#   scripts/install-local.sh                     installs ~/.local/bin/alz
#   PREFIX=/opt/alz scripts/install-local.sh     installs /opt/alz/bin/alz
#   BIN_DIR=~/bin scripts/install-local.sh       installs ~/bin/alz
#
# Needs Node.js 22.12 or newer, npm and a POSIX shell (macOS or Linux).
set -eu

die() {
  printf 'install-local: %s\n' "$*" >&2
  exit 1
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d ' ' -f 1
  else
    shasum -a 256 "$1" | cut -d ' ' -f 1
  fi
}

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
prefix=${PREFIX:-$HOME/.local}
bin_dir=${BIN_DIR:-$prefix/bin}
target=$bin_dir/alz
bundle=$root/dist/alz-standalone.mjs

command -v node >/dev/null 2>&1 || die "Node.js 22.12 or newer is required"
node -e 'const [major, minor] = process.versions.node.split(".").map(Number);
process.exit(major > 22 || (major === 22 && minor >= 12) ? 0 : 1)' ||
  die "Node.js 22.12 or newer is required (found $(node --version))"

if [ -L "$target" ]; then
  die "$target is a symlink, probably from an npm global install. Remove it first:
  npm uninstall --global --prefix \"$prefix\" agent-layerz    (or: rm \"$target\")"
fi
if [ -e "$target" ] && [ ! -f "$target" ]; then
  die "$target exists and is not a regular file"
fi

cd "$root"
[ -d node_modules ] || npm ci --no-audit --no-fund
npm run --silent build:standalone

mkdir -p "$bin_dir"
staged=$(mktemp "$bin_dir/.alz.XXXXXX")
trap 'rm -f "$staged"' EXIT
cp "$bundle" "$staged"
chmod 755 "$staged"
mv -f "$staged" "$target"
trap - EXIT

built=$(sha256 "$bundle")
installed=$(sha256 "$target")
[ "$built" = "$installed" ] || die "SHA-256 of $target does not match the build"
"$target" --help >/dev/null || die "$target does not run"

revision=$(git -C "$root" describe --always --dirty 2>/dev/null || echo unknown)
printf 'installed %s\n  version  %s (git %s)\n  sha256   %s\n' \
  "$target" "$("$target" --version)" "$revision" "$installed"
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) printf 'note: %s is not on PATH\n' "$bin_dir" ;;
esac
