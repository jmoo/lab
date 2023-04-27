# prmary linux workstation

{ config, pkgs, lib, ... }:

with lib;
with builtins;

{
  imports = [ ../home.nix ];

  lab.cinnamon.enable = true;
  lab.guake.enable = true;

  # Env vars
  home.sessionVariables = {
    NIX_PATH =
      "home-manager=/home/$USER/.nix-defexpr/channels/home-manager:nixpkgs=/home/$USER/.nix-defexpr/channels/nixos-22.11:$NIX_PATH";
    DOCKER_HOST = "unix:///run/user/1000/docker.sock";
    GCC_COLORS =
      "error=01;31:warning=01;35:note=01;36:caret=01;32:locus=01:quote=01";
  };

  home.packages = with pkgs; [ xdotool dconf2nix weechat ];

  lab.shell.aliases = {
    chat =
      "rlwrap /home/jmoore/Repos/relymd/platform/bin/relymd chat ~/chat.txt --auto-save";
  };

  lab.shell.init = ''
    # Add relymd aliases
    if [ -e ~/Repos/relymd/platform/bin/relymd ]; then
      . ~/Repos/relymd/platform/bin/relymd shim --global rmd relymd color no-color
      . ~/Repos/relymd/platform/bin/relymd shim node npm pnpm rush pg redis mgrep turbo tsc eslint nx
      . ~/Repos/relymd/platform/bin/relymd completions
    fi
  '';
}
