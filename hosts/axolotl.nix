{ lib, ... }:
let
  inherit (lib) mkDefault;
in
{
  lab.hosts.axolotl = {
    direnv.enable = true;
    ghostty.enable = true;
    hyprland.enable = true;

    nixos = {
      enable = true;
      eval = mkDefault false;

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

            networkmanager.enable = true;

            useDHCP = mkDefault true;
          };

          programs.wireshark.enable = true;

          security.rtkit.enable = true;

          services = {
            pipewire = {
              alsa = {
                enable = true;
                support32Bit = true;
              };
              enable = true;
              pulse.enable = true;
            };

            printing.enable = true;

            pulseaudio.enable = false;
          };

          users.users.jmoore = {
            description = "John Moore";
            extraGroups = [
              "networkmanager"
              "wheel"
              "dialout"
              "input"
            ];
            home = "/home/jmoore";
            isNormalUser = true;
            name = "jmoore";
          };
        };

      system = "x86_64-linux";
    };

    shell.enable = true;
    source = "/home/jmoore/Repos/jmoore/home";
    ssh.enable = true;
    user = "jmoore";
    vscode.enable = true;
  };
}
