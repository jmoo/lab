#!/usr/bin/env bash
# nix-deps: nix openssh

badger=${1:-badger}
host=${2:-$(hostname)}
repo=~/Repos/jmoo/lab

echo "Building badger home config..."
out=$(nix build "${repo}#homeConfigurations.badger.activationPackage" \
  --print-out-paths \
  --no-link)

# Bootstrap the launcher first. proot-distro inherits Termux's $HOME, so
# proot-badger has to set HOME itself (via --env) for nix/activation to target
# /home/jmoore. The on-device copy is whatever the *last* deploy left, so ship
# the current one before we rely on it. Written from the Termux side because
# proot can't reliably write its own bind-mounted ~/.shortcuts.
echo "Updating proot-badger launcher on ${badger}..."
ssh -p 8022 "jmoore@${badger}" \
  'cat > "$HOME/.shortcuts/proot-badger" && chmod 755 "$HOME/.shortcuts/proot-badger"' \
  <"${repo}/scripts/proot-badger.bash"

# `script` allocates a pty (so proot can sanitize its stdio bindings); `-e`
# returns the child's exit code so a failed copy/activate surfaces instead of
# being silently reported as success.
echo "Copying ${out} to ${badger} ..."
ssh -p 8022 "jmoore@${badger}" \
  "script -qe /dev/null -c \"\$HOME/.shortcuts/proot-badger 'nix copy --no-check-sigs --from ssh-ng://jmoore@${host} ${out} && stat ${out}'\""

echo "Activating ${out} ..."
ssh -p 8022 "jmoore@${badger}" "script -qe /dev/null -c \"\$HOME/.shortcuts/proot-badger ${out}/activate\""
