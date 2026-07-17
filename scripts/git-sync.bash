#!/usr/bin/env bash
# nix-deps: git

repo=$1
cd "$repo"
git add -A
git diff --cached --quiet || git commit -m "sync: $(date '+%Y-%m-%d %H:%M')"
git pull --no-rebase --no-edit
git push
