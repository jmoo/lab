{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeAll;
  inherit (lib') mkEnableOption mkIf;

  # Home-manager vscode configuration. `flags` carries the per-host toggles
  # (nix/python/rust/webdev) resolved from the flake-parts options.
  home =
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
      imports = [ ../pkgs/vscode-nix-extensions/home-manager.nix ];

      config = lib.mkMerge [
        {
          home.packages = with pkgs; [ shellcheck ];

          programs.vscode = {
            enable = true;

            nixExtensions.default = {
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

              iconThemes = {
                jmoo-dark-icons.path = ../resources/jmoo-dark-icons.json;
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
                  key = "ctrl+cmd+p";
                  command = "workbench.action.openRecent";
                }
                {
                  key = "ctrl+alt+p";
                  command = "workbench.action.openRecent";
                }
                {
                  key = "alt+enter";
                  command = "editor.action.quickFix";
                  when = "editorHasCodeActionsProvider && textInputFocus && !editorReadonly";
                }
                {
                  key = "alt+cmd+l";
                  command = "editor.action.formatDocument";
                  when = "editorHasDocumentFormattingProvider && editorTextFocus && !editorReadonly && !inCompositeEditor";
                }
                {
                  key = "ctrl+alt+l";
                  command = "editor.action.formatDocument";
                  when = "editorHasDocumentFormattingProvider && editorTextFocus && !editorReadonly && !inCompositeEditor";
                }
                {
                  key = "ctrl+r";
                  command = "editor.action.startFindReplaceAction";
                  when = "editorFocus || editorIsOpen";
                }
                {
                  key = "cmd+r";
                  command = "editor.action.startFindReplaceAction";
                  when = "editorFocus || editorIsOpen";
                }
              ];

              userSettings = {
                "cSpell.words" = lib.mkDefault (builtins.fromJSON (builtins.readFile ../dictionary.json));

                extensions.autoUpdate = false;
                "extensions.autoCheckUpdates" = false;

                "editor.fontFamily" = "Ubuntu Mono";
                "editor.fontSize" = 14;
                "editor.lineHeight" = 1.2;
                "editor.semanticHighlighting.enabled" = true;
                "editor.semanticTokenColorCustomizations" = {
                  enabled = true;
                  rules = {
                    "*.mutable" = {
                      "underline" = false;
                    };
                  };
                };

                "files.autoSave" = "afterDelay";

                files.associations = {
                  "*.json" = "jsonc";
                };

                git.openRepositoryInParentFolders = "always";

                "scm.countBadge" = "off";

                terminal.integrated.fontFamily = "MesloLGS NF";
                terminal.integrated.defaultProfile.osx = "zsh";
                terminal.external.osxExec = "iTerm.app";
                terminal.integrated.fontSize = 13;

                "update.mode" = "none";

                "window.customTitleBarVisibility" = "auto";
                "window.titleBarStyle" = "custom";

                "workbench.activityBar.location" = "top";
                "workbench.sideBar.location" = "left";
                "workbench.colorTheme" = "jmoo-dark";
                "workbench.iconTheme" = "jmoo-dark-icons";
                "workbench.tree.indent" = 20;

                "[css]" = {
                  "editor.defaultFormatter" = "vscode.css-language-features";
                };

                "[jsonc]" = {
                  "editor.defaultFormatter" = "vscode.json-language-features";
                };

                "[python]" = {
                  "editor.defaultFormatter" = "charliermarsh.ruff";
                };
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
              nix.enableLanguageServer = true;
              nix.serverPath = lsp.meta.mainProgram;
              nix.serverSettings.nil.formatting.command = [ formatter.meta.mainProgram ];
            };
          };
        })

        # Webdev
        (lib.mkIf flags.webdev.enable {
          programs.vscode.profiles.default.extensions = with pkgs.vscode-extensions; [
            dbaeumer.vscode-eslint
            esbenp.prettier-vscode
          ];
        })

        # Python
        (lib.mkIf flags.python.enable {
          programs.vscode.profiles.default.extensions = with pkgs.vscode-extensions; [
            ms-python.python
            charliermarsh.ruff
          ];
        })
      ];
    };
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
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

      config = mkIf config.vscode.enable (
        homeAll (home {
          inherit (config.vscode)
            nix
            python
            rust
            webdev
            ;
        })
      );
    }
  );
}
