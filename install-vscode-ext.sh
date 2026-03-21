#!/bin/sh
set -e

DEST="$HOME/.vscode/extensions/vscode-wst"

rm -rf "$DEST"
cp -r "$(dirname "$0")/vscode-wst" "$DEST"

echo "Installed to $DEST — restart VS Code to activate."
