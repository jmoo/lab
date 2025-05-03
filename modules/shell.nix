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
    enable = mkEnableOption "Enable default shell home-manager configuration";

    # Deprecated, no longer needed.
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
        ls = "ls --color";
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
        fzf
      ];
    };

    programs = {
      bash = {
        enable = true;
        enableCompletion = !pkgs.stdenv.isDarwin;
        shellAliases = config.lab.shell.aliases;

        bashrcExtra = ''
          case "$TERM" in
              xterm-color|*-256color) color_prompt=yes;;
          esac

          if [ "$color_prompt" = yes ]; then
              PS1='\[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ '
          else
              PS1='\u@\h:\w\$ '
          fi

          unset color_prompt force_color_prompt

          case "$TERM" in
          xterm*|rxvt*)
              PS1="\[\e]0;\u@\h: \w\a\]$PS1"
              ;;
          *)
              ;;
          esac

          ${config.lab.shell.init}
        '';

        shellOptions =
          [
            "histappend"
            "checkwinsize"
            "extglob"
          ]
          ++ (
            if pkgs.stdenv.isDarwin then
              [ ]
            else
              [
                "globstar"
                "checkjobs"
              ]
          );
      };

      fzf = {
        enable = true;
        enableZshIntegration = true;
      };

      starship = {
        enable = true;
        settings = {
          line_break.disabled = true;
          right_format = "$time$status";
          git_status.disabled = true;
        };
      };

      zsh = {
        enable = true;
        autocd = true;
        dotDir = ".config/zsh";
        autosuggestion.enable = true;
        enableCompletion = true;
        shellAliases = config.lab.shell.aliases;

        initContent = ''
          if [ -f /etc/zshrc ] && ! command -v nix > /dev/null 2> /dev/null; then
            source /etc/zshrc
          fi

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
