#!/bin/sh
# Makes every PNG of the two apps' icons from their SVG sources; run it
# after changing one. 16 and 24 px have a simpler drawing of their own.
# Needs rsvg-convert.
set -eu
cd "$(dirname "$0")/.."

render() { # source, size, target
  rsvg-convert -w "$2" -h "$2" "$1" -o "$3"
}

app=app/src-tauri/icons
light=tui/icons

for size in 16 24; do
  render "$app/logo-small.svg" "$size" "$app/${size}x$size.png"
  render "$light/light-small.svg" "$size" "$light/${size}x$size.png"
done
for size in 32 48 64 128 256; do
  render "$app/logo.svg" "$size" "$app/${size}x$size.png"
  render "$light/light.svg" "$size" "$light/${size}x$size.png"
done
render "$app/logo.svg" 512 "$app/icon.png"
render "$light/light.svg" 512 "$light/512x512.png"
render "$app/logo.svg" 64 app/public/icon.png
