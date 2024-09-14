# Shared shell config for bash and zsh so I don't have to configure them twice for simple things like
# aliases and basic init scripts.
{ config, pkgs, lib, ... }:

with lib;
with builtins;

let
  powerlevel10k-media =
    import ../pkgs/powerlevel10k-media.nix { inherit lib pkgs; };
in {
  options.lab.shell = {
    enable = mkEnableOption "shell";

    aliases = mkOption {
      type = types.attrsOf types.str;
      default = { };
    };

    init = mkOption {
      type = types.lines;
      default = "";
    };
  };

  config.lab.shell.aliases = mkIf config.lab.shell.enable {
    sl = "ls";
    ls = "ls";
    l = "ls";
    la = "ls -la";
    ll = "ls -laF";
    ip = "ip --color=auto";
    grep = "grep --color=auto";
    fgrep = "fgrep --color=auto";
    egrep = "egrep --color=auto";
    homelab = ". $HOMELAB_ROOT/bin/homelab";
  };

  config.home.packages = with pkgs;
    mkIf config.lab.shell.enable [
      rlwrap
      zsh-powerlevel10k
      powerlevel10k-media
      fzf
    ];

  config.programs.bash = mkIf config.lab.shell.enable {
    enable = true;
    enableCompletion = !pkgs.stdenv.isDarwin;
    shellAliases = config.lab.shell.aliases;

    bashrcExtra = ''
      ${builtins.readFile ../resources/dotfiles/bashrc}

      ${config.lab.shell.init}
    '';

    shellOptions = [
      # Append to history file rather than replacing it.
      "histappend"

      # check the window size after each command and, if
      # necessary, update the values of LINES and COLUMNS.
      "checkwinsize"

      # Extended globbing.
      "extglob"

    ] ++ (if pkgs.stdenv.isDarwin then
      [ ]
    else [
      # Extended globbing.
      "globstar"

      # Warn if closing shell with running jobs.
      "checkjobs"
    ]);
  };

  config.programs.zsh = mkIf config.lab.shell.enable {
    enable = true;
    autocd = true;
    dotDir = ".config/zsh";
    enableAutosuggestions = true;
    enableCompletion = true;
    shellAliases = config.lab.shell.aliases;

    initExtra = ''
      ${builtins.readFile ../resources/dotfiles/zshrc}
      ${builtins.readFile ../resources/dotfiles/p10k}

      ${if pkgs.stdenv.isDarwin then ''
        # Fix home/end keys
        bindkey '\e[H'    beginning-of-line
        bindkey '\e[F'    end-of-line
      '' else
        ""}

      ${config.lab.shell.init}
    '';
  };

  config.lab.shell.init = ''
    if ! command -v nix > /dev/null 2> /dev/null; then
      export PATH="$PATH:/nix/var/nix/profiles/default/bin"
    fi

    mnx() {
      nix \
        --extra-experimental-features nix-command \
        --extra-experimental-features flakes \
        --option warn-dirty false \
        develop \
          "path:$HOMELAB_ROOT/projects/mononix" -c mononix "$@"
    }

    find-up () {
      path=$(pwd)
      while [[ "$path" != "" && ! -e "$path/$1" ]]; do
        path=''${path%/*}
      done
      echo "$path"
    }

    sublime() {
      local cmd="sublime_text"
      local args=()

      local flake="$(find-up flake.nix)"
      local name="$(basename "$(pwd)")"

      if [[ "$#" = "0" ]] && [[ "$flake" != "" ]]; then
        cmd="nix"
        args=(develop -c sublime_text)
      fi

      if [ "$#" -gt 0 ]; then
        args+=("$@")

      elif [ -f ~/Repos/homelab/projects/"$name".sublime-project ]; then
        args+=(--project ~/Repos/homelab/projects/"$name".sublime-project)

      else
        args+=(".")
      fi

      $cmd "''${args[@]}" &
    }

    kill() {
      if [ "$#" -gt 0 ]; then
        command kill "$@";
      else
        command kill -9 $(ps aux | fzf | sed -E 's/^[^0-9]+([0-9]+).+$/\1/');
      fi
    }
  '';
}
