#!/usr/bin/env bash
set -euo pipefail

# fills the SHA256_ placeholders in the Homebrew formula and the winget installer manifest from
# the .sha256 files attached to a release, writing the finished files to an output folder so the
# templates in this repository keep their placeholders.
#
#   packaging/fill-checksums.sh v1.0.0 /path/to/output
#
# needs the GitHub CLI, logged in to an account that can read zacesshq/scenes.

tag="${1:?usage: fill-checksums.sh <tag> <output folder>}"
out="${2:?usage: fill-checksums.sh <tag> <output folder>}"
here="$(cd "$(dirname "$0")" && pwd)"
sums="$(mktemp -d)"
trap 'rm -rf "$sums"' EXIT

gh release download "$tag" --repo zacesshq/scenes --pattern '*.sha256' --dir "$sums"

mkdir -p "$out/homebrew" "$out/winget"
cp "$here/homebrew/zacess-scenes.rb" "$out/homebrew/"
cp "$here/winget/"*.yaml "$out/winget/"

for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl x86_64-pc-windows-msvc; do
    case "$target" in
        *windows*) file="zacess-scenes-$tag-$target.zip.sha256" ;;
        *) file="zacess-scenes-$tag-$target.tar.gz.sha256" ;;
    esac
    sum="$(awk '{print $1}' "$sums/$file")"
    if [ "${#sum}" -ne 64 ]; then
        printf 'No 64 character sum in %s\n' "$file" >&2
        exit 1
    fi
    # the placeholder is the target triple in upper case with hyphens as underscores
    placeholder="SHA256_$(printf '%s' "$target" | tr 'a-z-' 'A-Z_')"
    sed -i.bak "s/$placeholder/$sum/" "$out/homebrew/zacess-scenes.rb" "$out/winget/Zacess.Scenes.installer.yaml"
    rm -f "$out/homebrew/zacess-scenes.rb.bak" "$out/winget/Zacess.Scenes.installer.yaml.bak"
done

# a placeholder sits after a quote in the formula and after a colon in the manifest; the header
# comments only name the pattern
if grep -rqE '"SHA256_|: SHA256_' "$out"; then
    printf 'A placeholder is still unfilled in %s\n' "$out" >&2
    exit 1
fi
printf 'Filled %s/homebrew/zacess-scenes.rb and %s/winget/\n' "$out" "$out"
