{ pkgs, inputs, ... }:
{
  imports = [
    ./nix.nix
  ];

  system.stateVersion = 5;
  services.nix-daemon.enable = true;

  nixpkgs.hostPlatform = "aarch64-darwin";
}
