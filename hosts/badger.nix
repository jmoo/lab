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
              mkdir -p "$HOME/.shortcuts" "$HOME/.termux/boot"

              # Real file — Termux can't follow symlinks into /nix/store
              install -m 755 ${../scripts/proot-badger.bash} "$HOME/.shortcuts/proot-badger"

              printf '%s\n' \
                '#!/data/data/com.termux/files/usr/bin/bash' \
                'sshd' \
                > "$HOME/.shortcuts/start-sshd"
              chmod +x "$HOME/.shortcuts/start-sshd"

              printf '%s\n' \
                '#!/data/data/com.termux/files/usr/bin/bash' \
                '$HOME/.shortcuts/proot-badger obsidian-sync' \
                > "$HOME/.shortcuts/obsidian-sync"
              chmod +x "$HOME/.shortcuts/obsidian-sync"

              # Termux:Boot — auto-starts Termux sshd on device boot
              printf '%s\n' \
                '#!/data/data/com.termux/files/usr/bin/bash' \
                'sshd' \
                > "$HOME/.termux/boot/start-services"
              chmod +x "$HOME/.termux/boot/start-services"
            '';

            packages = [
              pkgs.openssh
              pkgs.proot-badger
            ];
          };

          programs.claude-code.enable = true;
        };
      system = "aarch64-linux";
    };
    obsidian.sync = {
      enable = true;
      vaults = [ "/home/jmoore/Repos/notes" ];
    };
    shell.enable = true;
    source = "github:jmoo/lab";
    user = "jmoore";
  };
}
