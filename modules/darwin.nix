{ pkgs, inputs, ... }: {
  imports = [
    inputs.home-manager.darwinModules.home-manager
    ./nix.nix
  ];

  system.stateVersion = 5;
  services.nix-daemon.enable = true;

  nixpkgs.hostPlatform = "aarch64-darwin";
}
