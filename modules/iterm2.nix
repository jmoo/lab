{
  config,
  pkgs,
  lib,
  ...
}:

with builtins;
with lib;

{
  options.lab.iterm2 = {
    enable = mkEnableOption "iterm2";
    package = mkOption {
      type = with types; nullOr package;
      default = pkgs.iterm2;
    };
  };

  config.home = mkIf config.lab.iterm2.enable {
    packages = mkIf (config.lab.iterm2.package != null) [ config.lab.iterm2.package ];

    file.iterm2-plist = {
      executable = false;
      source = ../resources/dotfiles/iterm2.plist;
      target = ".config/iterm2/com.googlecode.iterm2.plist";
    };
  };
}
