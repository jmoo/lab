#!/usr/bin/env bash
# nix-deps: nix openssh

badger=${1:-badger}
host=${2:-$HOSTNAME}

out=$(nix build ~/Repos/jmoo/lab#homeConfigurations.badger.activationPackage \
  --print-out-paths \
  --no-link)

ssh -t -p 8022 "jmoore@${badger}" \
  "proot-distro login debian --user jmoore -- bash -c 'source ~/.profile && nix copy --no-check-sigs --from ssh-ng://jmoore@${host} ${out} && ${out}/activate'"
