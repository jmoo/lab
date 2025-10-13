{ pkgs, lib, ... }:
let
  inherit (lib) mkForce;
in
{
  imports = [
    ../../modules/asahi.nix
    ./common.nix
  ];

  environment.systemPackages = with pkgs; [
    brave
    gparted
    lm_sensors
    obsidian
  ];

  home-manager.users.jmoore = {
    programs.ghostty.settings.theme = mkForce "Bright Lights";
  };

  lab = {
    asahi.peripheralFirmwareHash = "sha256-mP4xKnC15rZO5+D+wexGrim/7WUg23BbjwWLDEIsrPg=";
    ghostty.enable = true;
    ssh.enable = true;
  };

  services = {
    tailscale.enable = true;
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

  fileSystems."/" = {
    device = "/dev/disk/by-uuid/a217390b-0365-49ae-b9fa-b33118f286d5";
    fsType = "ext4";
  };

  fileSystems."/boot" = {
    device = "/dev/disk/by-uuid/7720-17F9";
    fsType = "vfat";
    options = [
      "fmask=0022"
      "dmask=0022"
    ];
  };
}
