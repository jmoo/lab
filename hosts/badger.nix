{ ... }:
{
  lab.hosts.badger = {
    direnv.enable = true;
    home = {
      eval = true;
      module =
        { pkgs, ... }:
        {
          home = {
            file.".shortcuts/proot-badger".source = "${pkgs.proot-badger}/bin/proot-badger";
            packages = [ pkgs.proot-badger ];
          };
        };
      system = "aarch64-linux";
    };
    shell.enable = true;
    source = "github:jmoo/lab";
    user = "jmoore";
  };
}
