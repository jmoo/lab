{ ... }:
{
  lab.hosts.badger = {
    direnv.enable = true;
    home = {
      eval = true;
      module =
        { pkgs, lib, ... }:
        {
          home = {
            activation.shortcuts = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
              mkdir -p "$HOME/.shortcuts"

              # Real file — Termux can't follow symlinks into /nix/store
              install -m 755 ${../scripts/proot-badger.bash} "$HOME/.shortcuts/proot-badger"

              # All other shortcuts run via proot-badger so they reach the nix store
              printf '%s\n' \
                '#!/data/data/com.termux/files/usr/bin/bash' \
                '$HOME/.shortcuts/proot-badger sshd' \
                > "$HOME/.shortcuts/start-sshd"
              chmod +x "$HOME/.shortcuts/start-sshd"
            '';
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
