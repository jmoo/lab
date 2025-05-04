{ ... }:
{
  imports = [
    ../../modules/nixos.nix
    ./hw
  ];

  lab = {
    source = "/home/jmoore/Repos/jmoo/lab";
    users = [ "jmoore" ];
    root = true;

    shell = {
      enable = true;
      root = true;
    };
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
}
