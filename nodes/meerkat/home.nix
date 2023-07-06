# arm macbook pro

{ config, pkgs, lib, ... }:

with lib;
with builtins;

{
  imports = [ ../../home.nix ];

  home.stateVersion = "23.05";

  lab.karabiner.enable = true;
  lab.nuphy75.enable = true;
  lab.astronvim.enable = true;
  lab.sublime.enable = true;
  lab.sublime-merge.enable = true;
  lab.youtube.enable = true;
  lab.iterm2.enable = true;

  lab.shell.aliases = {
    vi = "nvim";
    vim = "nvim";
  };

  home.packages = with pkgs; [
    weechat
    git
    gh
    ubuntu_font_family
    btop
    binwalk
    radare2
  ];

  lab.shell.init = ''
    # NOTE: homebrew reorders your PATH! Make changes to path after homebrew init.

    # homebrew
    if [ -e /usr/local/bin/brew ]; then
      eval "$(/usr/local/bin/brew shellenv)"
    fi

    # relymd
    if [ -e ~/Repos/relymd/platform/bin/relymd ]; then
      . ~/Repos/relymd/platform/bin/relymd --with-extra-tools shim --global rmd
    fi

    # nord-util
    export PATH=$PATH:/Users/jmoore/Repos/homelab/projects/nord-utils/target/debug:/Users/jmoore/Repos/homelab/projects/nord-utils/libnord-cli/bin

    # Fix path ordering to prefer nix profile binaries
    # https://gist.github.com/Linerre/f11ad4a6a934dcf01ee8415c9457e7b2
    # https://github.com/NixOS/nix/issues/4169
    export PATH=~/.nix-profile/bin:$PATH
  '';
}
