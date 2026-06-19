{ ... }:
{
  lab.hosts.badger = {
    direnv.enable = true;
    home = {
      eval = true;
      system = "aarch64-linux";
    };
    shell.enable = true;
    source = "github:jmoo/lab";
    user = "jmoore";
  };
}
