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
            file = {
              ".shortcuts/proot-badger".source = "${pkgs.proot-badger}/bin/proot-badger";
              ".shortcuts/start-sshd" = {
                executable = true;
                text = ''
                  #!/usr/bin/env bash
                  sshd
                '';
              };
            };
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
