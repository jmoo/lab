#!/usr/bin/env nix
#!nix shell --ignore-environment nixpkgs#cacert nixpkgs#coreutils nixpkgs#curl nixpkgs#bash --command bash

# Refresh the vendored claude-code release manifest (which pins the version +
# per-platform checksums the overlay builds from). Run with no arguments to
# track the latest release, or pass an explicit version:
#
#   ./pkgs/claude-code/update.sh            # latest
#   ./pkgs/claude-code/update.sh 2.1.220    # specific version
#
# Delete this whole directory + the overlay override once nixpkgs ships a
# claude-code new enough for the models you need (see overlay.nix).

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

BASE_URL="https://storage.googleapis.com/claude-code-dist-86c565f3-f756-42ad-8dfa-d59b1c096819/claude-code-releases"

VERSION="${1:-$(curl -fsSL "$BASE_URL/latest")}"

curl -fsSL "$BASE_URL/$VERSION/manifest.json" --output manifest.json

echo "claude-code manifest updated to $VERSION"
