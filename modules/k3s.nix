{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule forLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.k3s.enable = mkEnableOption "k3s nixos configuration";

      config = mkIf config.k3s.enable (
        forLinux (
          { config, ... }:
          {
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
          }
        )
      );
    }
  );
}
