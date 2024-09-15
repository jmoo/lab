# Shared shell config for bash and zsh so I don't have to configure them twice for simple things like
# aliases and basic init scripts.
{
  config,
  pkgs,
  lib,
  ...
}:

with lib;
with builtins;

{
  options.lab.shell = {
    enable = mkEnableOption "shell";

    # Deprecated, no longer needed. Use 
    aliases = mkOption {
      type = types.attrsOf types.str;
      default = { };
    };

    init = mkOption {
      type = types.lines;
      default = "";
    };
  };

  config = mkIf config.lab.shell.enable {
    lab.shell.init = ''
      if ! command -v nix > /dev/null 2> /dev/null; then
        export PATH="$PATH:/nix/var/nix/profiles/default/bin"
      fi

      find-up () {
        path=$(pwd)
        while [[ "$path" != "" && ! -e "$path/$1" ]]; do
          path=''${path%/*}
        done
        echo "$path"
      }

      kill() {
        if [ "$#" -gt 0 ]; then
          command kill "$@";
        else
          command kill -9 $(ps aux | fzf | sed -E 's/^[^0-9]+([0-9]+).+$/\1/');
        fi
      }
    '';

    home = {
      shellAliases = {
        sl = "ls";
        ls = "ls";
        l = "ls";
        la = "ls -la";
        ll = "ls -laF";
        ip = "ip --color=auto";
        grep = "grep --color=auto";
        fgrep = "fgrep --color=auto";
        egrep = "egrep --color=auto";
      };

      packages = with pkgs; [
        rlwrap
        zsh-powerlevel10k
        powerlevel10k-media
        fzf
      ];
    };

    programs = {
      bash = {
        enable = true;
        enableCompletion = !pkgs.stdenv.isDarwin;
        shellAliases = config.lab.shell.aliases;

        bashrcExtra = ''
          ${builtins.readFile ../resources/dotfiles/bashrc}

          ${config.lab.shell.init}
        '';

        shellOptions =
          [
            # Append to history file rather than replacing it.
            "histappend"

            # check the window size after each command and, if
            # necessary, update the values of LINES and COLUMNS.
            "checkwinsize"

            # Extended globbing.
            "extglob"
          ]
          ++ (
            if pkgs.stdenv.isDarwin then
              [ ]
            else
              [
                # Extended globbing.
                "globstar"

                # Warn if closing shell with running jobs.
                "checkjobs"
              ]
          );
      };

      zsh = {
        enable = true;
        autocd = true;
        dotDir = ".config/zsh";
        autosuggestion.enable = true;
        enableCompletion = true;
        shellAliases = config.lab.shell.aliases;

        initExtra = ''
          ${builtins.readFile ../resources/dotfiles/zshrc}
          ${builtins.readFile ../resources/dotfiles/p10k}

          ${optionalString pkgs.stdenv.isDarwin ''
            # Fix home/end keys
            bindkey '\e[H'    beginning-of-line
            bindkey '\e[F'    end-of-line
          ''}

          ${config.lab.shell.init}
        '';
      };
    };
  };
}
