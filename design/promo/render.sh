#!/bin/sh
# Renders the README images (English and Indonesian) with Chrome:
# the header at 2560x1280 and the performance card at 2560x1200.
cd "$(dirname "$0")"
chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
shot() { # page, width, height, output
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=2 \
    --window-size="$2,$3" --virtual-time-budget=5000 --screenshot="$PWD/$4" \
    "file://$PWD/$1" 2>/dev/null
}
only=${1:-all}
for lang in en id; do
  suffix=""
  [ "$lang" = id ] && suffix=".id"
  [ "$only" = all ] || [ "$only" = header ] && shot "header.html?lang=$lang" 1280 640 "../header$suffix.png"
  [ "$only" = all ] || [ "$only" = performance ] && shot "performance.html?lang=$lang" 1280 600 "../performance$suffix.png"
done
