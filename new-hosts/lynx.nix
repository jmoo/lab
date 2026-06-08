{ ... }:
{
  lab.hosts.lynx = {
    home.enable = true;

    nixos = {
      enable = true;
      system = "x86_64-linux";
    };
  };
}
