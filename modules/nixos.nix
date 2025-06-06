{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
with lib;
{
  imports = [
    inputs.home-manager.nixosModules.home-manager
    ./common.nix
    ./greetd.nix
    ./k3s.nix
    ./ssh.nix
  ];

  config = mkMerge [
    {
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

      lab.common = {
        home.stateVersion = mkDefault config.system.stateVersion;
        programs.home-manager.enable = false;
      };

      system.stateVersion = mkDefault "25.05";
      time.timeZone = "America/New_York";
    }

    # Hyprland
    (mkIf config.lab.hyprland.enable {
      environment.systemPackages = with pkgs; [
        kdePackages.qtwayland
        kdePackages.qtsvg
      ];

      lab.hyprland.common = {
        nvidia = elem "nvidia" config.services.xserver.videoDrivers;
        uwsm = true;
      };

      programs = {
        hyprland = {
          enable = true;
          withUWSM = true;
        };
        hyprlock.enable = true;
        xwayland.enable = true;
      };
    })

    # Shell configuration
    (mkIf config.lab.shell.enable {
      environment.shellInit = "unset __HM_SESS_VARS_SOURCED; [[ -e ~/.profile ]] && . ~/.profile";

      lab = {
        common.home.shellAliases.switch = mkDefault "sudo nixos-rebuild switch --flake ${config.lab.source}#${config.lab.name}";
        shell.common = {
          enable = mkDefault true;
        };
      };

      programs.zsh.enable = true;
      users.defaultUserShell = pkgs.zsh;
    })

    # Network Manager
    (mkIf config.networking.networkmanager.enable {
      lab.common = {
        services.network-manager-applet.enable = mkDefault true;

        systemd.user.services = {
          network-manager-applet.Unit = {
            After = [ "graphical-session.target" ];
            PartOf = [ "graphical-session.target" ];
          };
        };
      };
    })

    # Tailscale
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
            sleep 10

            # check if we are already authenticated to tailscale
            status="$(${tailscale}/bin/tailscale status -json | ${jq}/bin/jq -r .BackendState)"
            if [[ "$status" == "Running" ]]; then # if so, then do nothing
              exit 0
            fi

            # otherwise authenticate with tailscale
            ${tailscale}/bin/tailscale up -authkey $(cat /etc/tailscale/key) ${optionalString exitNode "--advertise-exit-node"} \
              --snat-subnet-routes=false \
              --advertise-routes=10.10.0.0/16
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
