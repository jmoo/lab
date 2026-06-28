#!/usr/bin/env bash
# nix-deps: nix openssh

badger=${1:-badger}

out=$(nix build "github:jmoo/lab#homeConfigurations.badger.activationPackage" \
  --print-out-paths \
  --no-link)

ssh -p 8022 "jmoore@${badger}" \
  "nix copy --from ssh-ng://jmoore@${HOSTNAME} ${out} && ${out}/activate"
