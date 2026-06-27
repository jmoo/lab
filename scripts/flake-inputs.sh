#!/usr/bin/env bash
# deps: nix jq

# Show all locked flake inputs with their short revision hashes.
nix flake metadata --json \
  | jq -r '.locks.nodes | to_entries[] | select(.value.locked != null) | "\(.key)\t\(.value.locked.rev[:8])"' \
  | sort
