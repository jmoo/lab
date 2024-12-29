{ inputs, lib, ... }: with lib;
{
  imports = [ 
    inputs.home-manager.nixosModules.home-manager
    ./nix.nix 
  ];

  system.stateVersion = mkDefault "25.05";
}
