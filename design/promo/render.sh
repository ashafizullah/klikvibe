#!/bin/sh
# Renders the README headers (English and Indonesian) at 2560x1280 with Chrome.
cd "$(dirname "$0")"
chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
for lang in en id; do
  out=../header.png
  [ "$lang" = id ] && out=../header.id.png
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=2 \
    --window-size=1280,640 --virtual-time-budget=5000 --screenshot="$PWD/$out" \
    "file://$PWD/header.html?lang=$lang" 2>/dev/null
done
