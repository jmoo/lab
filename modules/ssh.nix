{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule forLinux;
  inherit (lib')
    mkEnableOption
    mkIf
    mkOption
    types
    ;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    let
      cfg = config.ssh;
    in
    {
      options.ssh = {
        enable = mkEnableOption "openssh nixos configuration";

        port = mkOption {
          description = "Port to use for ssh";
          type = types.number;
          default = 22;
        };

        users = mkOption {
          description = "Enable ssh for these users (defaults to all normal users)";
          type = with types; nullOr (listOf str);
          default = null;
        };
      };

      config = mkIf cfg.enable (
        forLinux (
          { config, lib, ... }:
          {
            networking.firewall.allowedTCPPorts = [
              cfg.port
            ];

            services.openssh = {
              enable = true;
              ports = [ cfg.port ];
              settings = {
                PasswordAuthentication = true;
                AllowUsers =
                  if cfg.users != null then
                    cfg.users
                  else
                    lib.mapAttrsToList (_: v: v.name) (lib.filterAttrs (_: v: v.isNormalUser) config.users.users);
                UseDns = true;
                X11Forwarding = false;
                PermitRootLogin = "no";
              };
            };
          }
        )
      );
    }
  );
}
