{ mkHome, ... }:
{
  imports = [ ../../modules/nixos.nix ];

  home-manager.users.jmoore = mkHome {
    lab = {
      direnv.enable = true;
      vscode.enable = true;
    };
  };

  lab.shell.enable = true;

  users.users.jmoore = {
    name = "jmoore";
    home = "/home/jmoore";
    isNormalUser = true;
    description = "John Moore";
    extraGroups = [
      "networkmanager"
      "wheel"
    ];
  };
}
