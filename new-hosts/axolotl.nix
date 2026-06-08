{ ... }:
{
  lab.hosts.axolotl = {
    home.enable = true;

    nixos = {
      enable = true;
      system = "x86_64-linux";
    };
  };
}
