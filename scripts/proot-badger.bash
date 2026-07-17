#!/data/data/com.termux/files/usr/bin/bash

USER="jmoore"
HOME_DIR="/home/$USER"
STORAGE="/storage/emulated/0"

# proot-distro inherits Termux's $HOME; force it to the debian user's home so
# nix, home-manager activation and friends operate on /home/jmoore, not on
# Termux's /data/data/.../home (which doesn't exist inside the container).
MOUNTS=(
  --env "HOME=$HOME_DIR"
  --bind "$STORAGE/Repos:$HOME_DIR/Repos"
  --bind "$STORAGE/Download:$HOME_DIR/Download"
  --bind "$STORAGE/Documents:$HOME_DIR/Documents"
  --bind "$STORAGE/Pictures:$HOME_DIR/Pictures"
  --bind "$HOME/.shortcuts:$HOME_DIR/.shortcuts"
  --bind "$HOME/.ssh:$HOME_DIR/.ssh"
  --bind "$HOME/.termux:$HOME_DIR/.termux"
)

if [ $# -gt 0 ]; then
  proot-distro login debian --user "$USER" "${MOUNTS[@]}" -- \
    bash -ic "$@"
else
  proot-distro login debian --user "$USER" "${MOUNTS[@]}"
fi
