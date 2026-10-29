#!/usr/bin/env bash
# regenerates the gallery stills in docs/images/scenes and the showcase frames in docs/frames
# from the program itself, so they always match what the scenes draw. CI runs this and fails
# when the committed files differ.
set -euo pipefail

cd "$(dirname "$0")/.."
cargo build --release --quiet
bin=target/release/zacess-scenes
[ -x "$bin" ] || bin="$bin.exe"

# a fixed config path keeps a developer's own settings and Lua scenes out of the gallery
export ZACESS_CONFIG="$PWD/target/snapshots-no-config/scenes.toml"
unset ZACESS_THEME ZACESS_MOTION ZACESS_REDUCED_MOTION XDG_STATE_HOME
export XDG_STATE_HOME="$PWD/target/snapshots-no-state"

stills=docs/images/scenes
frames=docs/frames
rm -rf "$stills" "$frames"
mkdir -p "$stills" "$frames"

for scene in $("$bin" --list-scenes | awk '{print $1}'); do
  "$bin" snapshot "$scene" --theme deep-contrast-dark --size 80x24 --time 6 --seed 1 \
    --out "$stills/$scene.svg"
done
"$bin" snapshot waves --state pass --theme deep-contrast-dark --size 80x24 --time 6 --seed 1 \
  --out "$stills/watch-pass.svg"
"$bin" snapshot storm --state fail --theme deep-contrast-dark --size 80x24 --time 6 --seed 1 \
  --out "$stills/watch-fail.svg"
"$bin" snapshot lanterns --theme light --size 80x24 --time 6 --seed 1 \
  --out "$stills/lanterns-light.svg"

for scene in koi-pond lanterns tea constellations fog traffic-jam drought zacess; do
  "$bin" snapshot "$scene" --theme deep-contrast-dark --size 80x24 --time 6 --seed 1 \
    --frames 72 --fps 12 --out "$frames/$scene.json"
done
