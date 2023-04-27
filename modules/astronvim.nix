{ config, pkgs, lib, ... }:

with lib;
with builtins;

let
  astronvim = fetchTree {
    type = "git";
    url = "https://github.com/AstroNvim/AstroNvim";
    rev = "291c43b0bdd551ebaf4de11e4f41ff7ddb38559a";
  };

  files = (pipe astronvim [
    (lib.filesystem.listFilesRecursive)
    (map (x: {
      name = ".config/nvim${
          unsafeDiscardStringContext (removePrefix "${astronvim}" x)
        }";
      value = x;
    }))
    (listToAttrs)
  ]) // {
    # override default astrovim config here
  };
in {
  options.lab.astronvim.enable = mkEnableOption "astronvim";

  config.home = mkIf config.lab.astronvim.enable {
    packages = with pkgs; [
      neovim
      ripgrep
      lazygit
      gdu
      bottom
      nodejs
      python311
    ];

    file = listToAttrs (map (file: {
      name = "astronvim_${file.name}";
      value = {
        executable = false;
        source = file.value;
        target = file.name;
      };
    }) (map (name: {
      inherit name;
      value = getAttr name files;
    }) (attrNames files)));
  };
}
