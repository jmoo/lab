{ lib, ... }:
let
  inherit (lib) mkDefault;
in
{
  lab.hosts.axolotl = {
    user = "jmoore";
    source = "/home/jmoore/Repos/jmoore/home";

    direnv.enable = true;
    ghostty.enable = true;
    hyprland.enable = true;
    shell.enable = true;
    ssh.enable = true;
    vscode.enable = true;

    nixos = {
      enable = true;
      eval = mkDefault false;
      system = "x86_64-linux";

      module =
        { pkgs, ... }:
        {
          environment.systemPackages = with pkgs; [
            git
            gparted
            vim
          ];

          networking = {
            hostName = "axolotl";

            networkmanager = {
              enable = true;
            };

            useDHCP = lib.mkDefault true;
          };

          programs = {
            wireshark.enable = true;
          };

          security.rtkit.enable = true;

          services = {
            pipewire = {
              enable = true;
              alsa.enable = true;
              alsa.support32Bit = true;
              pulse.enable = true;
            };

            printing.enable = true;

            pulseaudio.enable = false;
          };

          users.users.jmoore = {
            name = "jmoore";
            home = "/home/jmoore";
            isNormalUser = true;
            description = "John Moore";
            extraGroups = [
              "networkmanager"
              "wheel"
              "dialout"
              "input"
            ];
          };
        };
    };
  };
}
