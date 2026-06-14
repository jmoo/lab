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
          default = 22;
          description = "Port to use for ssh";
          type = types.int;
        };

        users = mkOption {
          default = null;
          description = "Enable ssh for these users (defaults to all normal users)";
          type = with types; nullOr (listOf str);
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
                AllowUsers =
                  if cfg.users != null then
                    cfg.users
                  else
                    lib.mapAttrsToList (_: v: v.name) (lib.filterAttrs (_: v: v.isNormalUser) config.users.users);
                PasswordAuthentication = false;
                PermitRootLogin = "no";
                UseDns = true;
                X11Forwarding = false;
              };
            };
          }
        )
      );
    }
  );
}
