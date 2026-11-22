#!/bin/sh
# Build the Rust alz from this checkout and install it as one regular
# executable: no symlink. Rerun it to update an existing install in place.
#
# Until cutover (TASK-55.9) the Rust binary installs as alz-rs, next to the
# Node alz from install-local.sh, because it does not have every command yet.
#
#   scripts/install-local-rust.sh                     installs ~/.local/bin/alz-rs
#   PREFIX=/opt/alz scripts/install-local-rust.sh     installs /opt/alz/bin/alz-rs
#   BIN_DIR=~/bin scripts/install-local-rust.sh       installs ~/bin/alz-rs
#   ALZ_NAME=alz scripts/install-local-rust.sh        replaces the alz command itself
#
# Needs the toolchain pinned in mise.toml; run it as `mise run install:local:rust`.
set -eu

die() {
  printf 'install-local-rust: %s\n' "$*" >&2
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
name=${ALZ_NAME:-alz-rs}
case $name in
  '' | */* | .*) die "ALZ_NAME must be a plain file name (got '$name')" ;;
esac
target=$bin_dir/$name

command -v cargo >/dev/null 2>&1 || die "cargo is not on PATH; run this through: mise run install:local:rust"

if [ -L "$target" ]; then
  die "$target is a symlink. Remove it first: rm \"$target\""
fi
if [ -e "$target" ] && [ ! -f "$target" ]; then
  die "$target exists and is not a regular file"
fi

replaced=''
if [ -f "$target" ]; then
  if head -c 64 "$target" | grep -q 'node'; then
    replaced='the Node build of alz'
  else
    replaced="$("$target" --version 2>/dev/null || echo 'an unknown build')"
  fi
fi

cd "$root"
cargo build --release --locked --package alz
built_path=$(cargo metadata --format-version 1 --no-deps --locked |
  sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')/release/alz
[ -f "$built_path" ] || die "the build did not produce $built_path"

mkdir -p "$bin_dir"
staged=$(mktemp "$bin_dir/.$name.XXXXXX")
trap 'rm -f "$staged"' EXIT
cp "$built_path" "$staged"
chmod 755 "$staged"
mv -f "$staged" "$target"
trap - EXIT

built=$(sha256 "$built_path")
installed=$(sha256 "$target")
[ "$built" = "$installed" ] || die "SHA-256 of $target does not match the build"
"$target" --help >/dev/null || die "$target does not run"

revision=$(git -C "$root" describe --always --dirty 2>/dev/null || echo unknown)
printf 'installed %s\n  version  %s (git %s)\n  sha256   %s\n' \
  "$target" "$("$target" --version)" "$revision" "$installed"
[ -z "$replaced" ] || printf '  previous %s\n' "$replaced"
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) printf 'note: %s is not on PATH\n' "$bin_dir" ;;
esac
