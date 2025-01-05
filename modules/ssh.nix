{ lib, config, ... }:
with lib;
{
  options.lab.ssh = {
    enable = mkEnableOption "Enable openssh nixos configuration";

    port = mkOption {
      description = "Port to use for ssh";
      type = types.number;
      default = 22;
    };

    users = mkOption {
      description = "Enable ssh for these users";
      type = with types; listOf str;
      default = mapAttrsToList (_: v: v.name) (filterAttrs (_: v: v.isNormalUser) config.users.users);
    };
  };

  config = mkIf config.lab.ssh.enable {
    networking.firewall.allowedTCPPorts = [
      config.lab.ssh.port
    ];

    services.openssh = {
      enable = true;
      ports = [ config.lab.ssh.port ];
      settings = {
        PasswordAuthentication = true;
        AllowUsers = config.lab.ssh.users;
        UseDns = true;
        X11Forwarding = false;
        PermitRootLogin = "no";
      };
    };
  };
}
