#!/data/data/com.termux/files/usr/bin/bash

USER="jmoore"
HOME_DIR="/home/$USER"
STORAGE="/storage/emulated/0"

MOUNTS=(
  --bind "$STORAGE/Repos:$HOME_DIR/Repos"
  --bind "$STORAGE/Download:$HOME_DIR/Download"
  --bind "$STORAGE/Documents:$HOME_DIR/Documents"
  --bind "$STORAGE/Pictures:$HOME_DIR/Pictures"
  --bind "$HOME/.shortcuts:$HOME_DIR/.shortcuts"
  --bind "$HOME/.ssh:$HOME_DIR/.ssh"
)

if [ $# -gt 0 ]; then
  proot-distro login debian --user "$USER" "${MOUNTS[@]}" -- \
    bash -c "source ~/.bashrc && $*"
else
  proot-distro login debian --user "$USER" "${MOUNTS[@]}"
fi
