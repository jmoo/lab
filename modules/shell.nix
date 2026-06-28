{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib')
    mkEnableOption
    mkIf
    mkDefault
    optionalString
    ;

  # Shared shell init snippet (was lab.shell.init).
  init = ''
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

  # Home-manager shell configuration.
  home =
    { pkgs, config, ... }:
    {
      home = {
        packages = with pkgs; [
          rlwrap
          fzf
          vim
        ];

        sessionVariables.EDITOR = "vim";

        shellAliases = {
          egrep = "egrep --color=auto";
          fgrep = "fgrep --color=auto";
          grep = "grep --color=auto";
          ip = "ip --color=auto";
          l = "ls";
          la = "ls -la";
          ll = "ls -laF";
          ls = "ls --color";
          sl = "ls";
        };
      };

      programs = {
        bash = {
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

            ${init}
          '';
          enable = true;
          enableCompletion = !pkgs.stdenv.isDarwin;

          shellOptions = [
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
            git_status.disabled = true;
            line_break.disabled = true;
            right_format = "$time$status";
          };
        };

        yazi = {
          enable = true;
          enableBashIntegration = true;
          enableZshIntegration = true;
          shellWrapperName = "d";
        };

        zsh = {
          autocd = true;
          autosuggestion.enable = true;
          dotDir = "${config.xdg.configHome}/zsh";
          enable = true;
          enableCompletion = true;

          initContent = ''
            if [ -f /etc/zshrc ] && ! command -v nix > /dev/null 2> /dev/null; then
              source /etc/zshrc
            fi

            ${optionalString pkgs.stdenv.isDarwin ''
              # Fix home/end keys
              bindkey '\e[H'    beginning-of-line
              bindkey '\e[F'    end-of-line
            ''}

            ${init}
          '';
        };
      };
    };
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    let
      switch = command: {
        home.shellAliases.switch = mkDefault "sudo ${command} switch --flake ${config.source}#${config.name}";
      };

      # Linux system-level shell wiring.
      linuxSystem =
        { pkgs, ... }:
        {
          programs.zsh.enable = true;
          users.defaultUserShell = pkgs.zsh;
        };

      nixosShellInit = {
        environment.shellInit = "unset __HM_SESS_VARS_SOURCED; [[ -e ~/.profile ]] && . ~/.profile";
      };
    in
    {
      config = mkIf config.shell.enable {
        asahi = {
          home.imports = [ (switch "nixos-rebuild") ];
          module.imports = [
            linuxSystem
            nixosShellInit
          ];
        };

        darwin = {
          home.imports = [ (switch "darwin-rebuild") ];
          module.programs.zsh.enable = true;
        };

        home.module = home;

        nixos = {
          home.imports = [ (switch "nixos-rebuild") ];
          module.imports = [
            linuxSystem
            nixosShellInit
          ];
        };
      };

      options.shell.enable = mkEnableOption "default shell home-manager configuration";
    }
  );
}
