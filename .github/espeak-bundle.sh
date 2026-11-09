#!/bin/bash
# Copies the system eSpeak (program, its own libraries, English voice data) into src-tauri/espeak,
# which the Linux build packs into the AppImage for the TV's voice-over.
set -euo pipefail
out=src-tauri/espeak
rm -rf "$out" && mkdir -p "$out/bin" "$out/lib"
cp /usr/bin/espeak-ng "$out/bin/"
# only eSpeak's own libraries; everything else (sound, C library) is already on any desktop Linux
ldd /usr/bin/espeak-ng | grep -oE '/\S*lib(espeak-ng|pcaudio|sonic)\S*' | xargs -I{} cp -L {} "$out/lib/"
data=$(dirname "$(find /usr/lib /usr/share -type d -name espeak-ng-data | head -1)")
cp -r "$data/espeak-ng-data" "$out/"
# English only keeps it small
find "$out/espeak-ng-data" -maxdepth 1 -name '*_dict' ! -name en_dict -delete
LD_LIBRARY_PATH="$out/lib" "$out/bin/espeak-ng" --path="$out" -v en-us -w /tmp/check.wav "voice check"
head -c 4 /tmp/check.wav | grep -q RIFF
du -sh "$out"
