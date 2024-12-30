{
  config,
  lib,
  pkgs,
  mkHome,
  ...
}:
with lib;
{
  imports = [
    ./home-manager.nix
    ./k3s.nix
    ./nix.nix
    ./ssh.nix
  ];

  options = {
    lab = {
      name = mkOption {
        type = types.str;
        default = config.networking.hostName;
      };

      source = mkOption {
        type = with types; nullOr str;
        default = "github:jmoo/lab";
      };
    };
  };

  config = mkMerge [
    {
      home-manager.users.root = mkHome {
        home.homeDirectory = "/root";
      };

      i18n.defaultLocale = "en_US.UTF-8";
      i18n.extraLocaleSettings = {
        LC_ADDRESS = "en_US.UTF-8";
        LC_IDENTIFICATION = "en_US.UTF-8";
        LC_MEASUREMENT = "en_US.UTF-8";
        LC_MONETARY = "en_US.UTF-8";
        LC_NAME = "en_US.UTF-8";
        LC_NUMERIC = "en_US.UTF-8";
        LC_PAPER = "en_US.UTF-8";
        LC_TELEPHONE = "en_US.UTF-8";
        LC_TIME = "en_US.UTF-8";
      };

      system.stateVersion = mkDefault "25.05";
      time.timeZone = "America/New_York";
    }

    (mkIf config.lab.hyprland.enable {
      environment.systemPackages = with pkgs; [
        kitty
      ];

      home-manager.common.lab.hyprland.enable = mkDefault true;
      programs.hyprland.enable = true;
    })

    (mkIf config.lab.shell.enable {
      home-manager.common = {
        lab.shell.enable = mkDefault true;
        home.shellAliases.switch = mkDefault "sudo nixos-rebuild switch --flake ${config.lab.source}#${config.lab.name}";
      };

      environment.shellInit = "unset __HM_SESS_VARS_SOURCED; [[ -e ~/.profile ]] && . ~/.profile";
      users.defaultUserShell = pkgs.zsh;
      programs.zsh.enable = true;
    })

    (mkIf config.services.tailscale.enable (
      let
        tailscale = config.services.tailscale.package;
        exitNode = config.services.tailscale.useRoutingFeatures == "server";
      in
      {
        environment.systemPackages = [
          config.services.tailscale.package
        ];

        networking.firewall.checkReversePath = mkIf exitNode "loose";
        systemd.services.tailscale-autoconnect = {
          description = "Automatic connection to Tailscale";
          serviceConfig.Type = "oneshot";

          script = with pkgs; ''
            # wait for tailscaled to settle
            sleep 2

            # check if we are already authenticated to tailscale
            status="$(${tailscale}/bin/tailscale status -json | ${jq}/bin/jq -r .BackendState)"
            if [ $status = "Running" ]; then # if so, then do nothing
              exit 0
            fi

            # otherwise authenticate with tailscale
            ${tailscale}/bin/tailscale up -authkey $(cat /etc/tailscale/key) ${optionalString exitNode "--advertise-exit-node"}
          '';

          after = [
            "network-pre.target"
            "tailscale.service"
          ];
          wants = [
            "network-pre.target"
            "tailscale.service"
          ];
          wantedBy = [ "multi-user.target" ];
        };
      }
    ))
  ];
}
