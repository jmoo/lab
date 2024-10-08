{ ... }:
{
  imports = [ ../../modules/nixos.nix ];

  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
    users.jmoore = _: {
      imports = [ ../../modules/home.nix ];

      home = {
        homeDirectory = "/home/jmoore";
        username = "jmoore";
      };
    };
  };

  programs.zsh.enable = true;

  users.users.jmoore = {
    name = "jmoore";
    home = "/home/jmoore";
  };
}
