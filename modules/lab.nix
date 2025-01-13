{ lib, ... }:
with lib;
{
  options = {
    lab = {
      name = mkOption {
        description = "Name of the nix configuration";
        type = types.str;
      };

      source = mkOption {
        description = "Flake uri of the jmoo/lab source";
        type = with types; nullOr str;
        default = "github:jmoo/lab";
      };
    };
  };
}
