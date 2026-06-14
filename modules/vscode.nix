{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib') mkEnableOption mkIf;

  # Home-manager vscode configuration. `flags` carries the per-host toggles
  # (nix/python/rust/webdev) resolved from the flake-parts options.
  mkHome =
    flags:
    {
      pkgs,
      lib,
      ...
    }:
    let
      formatter = pkgs.nixfmt;
      lsp = pkgs.nil;
    in
    {
      config = lib.mkMerge [
        {
          home.packages = with pkgs; [ shellcheck ];

          programs.vscode = {
            enable = true;

            nixExtensions.default = {
              iconThemes.jmoo-dark-icons.path = ../resources/jmoo-dark-icons.json;

              paths = [
                {
                  from = ../resources/icons;
                  to = "./nix/store/icons";
                }
              ];

              themes = {
                jmoo-dark = {
                  path = ../resources/jmoo-dark.json;
                  uiTheme = "vs-dark";
                };

                yarra-valley = {
                  path = ../resources/yarra-valley.json;
                  uiTheme = "vs-dark";
                };
              };
            };

            profiles.default = {
              extensions = with pkgs.vscode-extensions; [
                ms-vscode-remote.remote-ssh
                ms-azuretools.vscode-docker
                usernamehw.errorlens
                streetsidesoftware.code-spell-checker
                tamasfe.even-better-toml
                mads-hartmann.bash-ide-vscode
                charliermarsh.ruff
                esbenp.prettier-vscode
              ];

              keybindings = [
                {
                  command = "workbench.action.openRecent";
                  key = "ctrl+cmd+p";
                }
                {
                  command = "workbench.action.openRecent";
                  key = "ctrl+alt+p";
                }
                {
                  command = "editor.action.quickFix";
                  key = "alt+enter";
                  when = "editorHasCodeActionsProvider && textInputFocus && !editorReadonly";
                }
                {
                  command = "editor.action.formatDocument";
                  key = "alt+cmd+l";
                  when = "editorHasDocumentFormattingProvider && editorTextFocus && !editorReadonly && !inCompositeEditor";
                }
                {
                  command = "editor.action.formatDocument";
                  key = "ctrl+alt+l";
                  when = "editorHasDocumentFormattingProvider && editorTextFocus && !editorReadonly && !inCompositeEditor";
                }
                {
                  command = "editor.action.startFindReplaceAction";
                  key = "ctrl+r";
                  when = "editorFocus || editorIsOpen";
                }
                {
                  command = "editor.action.startFindReplaceAction";
                  key = "cmd+r";
                  when = "editorFocus || editorIsOpen";
                }
              ];

              userSettings = {
                "[css]" = {
                  "editor.defaultFormatter" = "vscode.css-language-features";
                };

                "[jsonc]" = {
                  "editor.defaultFormatter" = "vscode.json-language-features";
                };

                "[python]" = {
                  "editor.defaultFormatter" = "charliermarsh.ruff";
                };

                "cSpell.words" = lib.mkDefault (builtins.fromJSON (builtins.readFile ../dictionary.json));

                "editor.fontFamily" = "Ubuntu Mono";
                "editor.fontSize" = 14;
                "editor.lineHeight" = 1.2;
                "editor.semanticHighlighting.enabled" = true;
                "editor.semanticTokenColorCustomizations" = {
                  enabled = true;
                  rules."*.mutable"."underline" = false;
                };

                "extensions.autoCheckUpdates" = false;
                extensions.autoUpdate = false;

                files.associations = {
                  "*.json" = "jsonc";
                };

                "files.autoSave" = "afterDelay";

                git.openRepositoryInParentFolders = "always";

                "scm.countBadge" = "off";

                terminal = {
                  external.osxExec = "iTerm.app";
                  integrated = {
                    defaultProfile.osx = "zsh";
                    fontFamily = "MesloLGS NF";
                    fontSize = 13;
                  };
                };

                "update.mode" = "none";

                "window.customTitleBarVisibility" = "auto";
                "window.titleBarStyle" = "custom";

                "workbench.activityBar.location" = "top";
                "workbench.colorTheme" = "jmoo-dark";
                "workbench.iconTheme" = "jmoo-dark-icons";
                "workbench.sideBar.location" = "left";
                "workbench.tree.indent" = 20;
              };
            };
          };
        }

        # Rust
        (lib.mkIf flags.rust.enable {
          programs.vscode.profiles.default.extensions = with pkgs.vscode-extensions; [
            rust-lang.rust-analyzer
          ];
        })

        # Nix
        (lib.mkIf flags.nix.enable {
          home.packages = [
            formatter
            lsp
          ];

          programs.vscode.profiles.default = {
            extensions = with pkgs.vscode-extensions; [ jnoortheen.nix-ide ];

            userSettings = {
              nix = {
                enableLanguageServer = true;
                serverPath = lsp.meta.mainProgram;
                serverSettings.nil.formatting.command = [ formatter.meta.mainProgram ];
              };
            };
          };
        })

        # Webdev
        (lib.mkIf flags.webdev.enable {
          programs.vscode.profiles.default.extensions = with pkgs.vscode-extensions; [
            dbaeumer.vscode-eslint
          ];
        })

        # Python
        (lib.mkIf flags.python.enable {
          programs.vscode.profiles.default.extensions = with pkgs.vscode-extensions; [
            ms-python.python
          ];
        })
      ];

      imports = [ ../pkgs/vscode-nix-extensions/home-manager.nix ];
    };
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      config = mkIf config.vscode.enable {
        home = mkHome {
          inherit (config.vscode)
            nix
            python
            rust
            webdev
            ;
        };
      };

      options.vscode = {
        enable = mkEnableOption "vscode home-manager configuration";
        nix.enable = mkEnableOption "nix language support" // {
          default = true;
        };
        python.enable = mkEnableOption "python language support" // {
          default = true;
        };
        rust.enable = mkEnableOption "rust language support" // {
          default = true;
        };
        webdev.enable = mkEnableOption "js/ts language support" // {
          default = true;
        };
      };
    }
  );
}
