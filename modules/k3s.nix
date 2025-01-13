{ config, lib, ... }:
with lib;
{
  options.lab.k3s = {
    enable = mkEnableOption "Enable k3s nixos configuration";
  };

  config = mkIf config.lab.k3s.enable {
    services.k3s = {
      enable = true;
      images = [
        config.services.k3s.package.airgapImages
      ];

      role = "server";
      extraFlags = toString [
        # "--debug" # Optionally add additional args to k3s
      ];
    };
  };
}
