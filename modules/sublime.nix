{ config, pkgs, lib, ... }:

with builtins;
with lib;

let
  json = (pkgs.formats.json { }).type;

  mkConfigFiles = extension: attrs:
    mapAttrs' (name: value: {
      name = "${name}.${extension}";
      value = {
        source = pkgs.writeTextFile {
          name = "${name}.${extension}";
          text = toJSON value;
        };

        target = "${config.lab.sublime.userDirectory}/${name}.${extension}";
      };
    }) attrs;

  mkAdapter = name: def: (
    if def.enable && config.lab.sublime.debugger.enable then
     {
       "sublime-debugger-${name}" = {
         recursive = true;
         source = def.package;
         target = "${config.lab.sublime.userDirectory}/../SublimeDebugger/data/${name}";
      };
    } 
    else {}
  );

in {
  options.lab.sublime = { 
    enable = mkEnableOption "sublime"; 

    userDirectory = mkOption {
      type = types.str;
      default = "Library/Application Support/Sublime Text/Packages/User";
    };

    packageDirectory = mkOption {
      type = types.str;
      default = "Library/Application Support/Sublime Text/Installed Packages";
    };

    package = mkOption {
      type = types.package;
      default = import ../packages/sublime.nix { inherit pkgs; };
    };

    packages = mkOption {
      type = types.listOf types.package;
      default = [
        (import ../packages/sublime-package-control.nix { inherit pkgs; })
        (import ../packages/sublime-lsp.nix { inherit pkgs; })
        (import ../packages/sublime-bash.nix { inherit pkgs; })
        (import ../packages/sublime-nix.nix { inherit pkgs; })
        (import ../packages/sublime-copilot.nix { inherit pkgs; })
        (import ../packages/sublime-rust-analyzer.nix { inherit pkgs; })
        (import ../packages/sublime-typescript.nix { inherit pkgs; })
        (import ../packages/sublime-toml.nix { inherit pkgs; })
        (import ../packages/sublime-sidebar-enhancements.nix { inherit pkgs; })
        (import ../packages/sublime-pylsp.nix { inherit pkgs; })
      ];
    };

    debugger = mkOption {
      type = types.submodule {
        options = {
          enable = mkOption {
            type = types.bool;
          };
          
          package = mkOption {
            type = types.package;
          };

          settings = mkOption {
            type = types.submodule { freeformType = json; };
          };

          lldb = mkOption {
            type = types.submodule {
              options = {
                enable = mkOption {
                  type = types.bool;
                };

                package = mkOption {
                  type = types.package;
                };

                settings = mkOption {
                  type = types.submodule { freeformType = json; };
                };
              };
            };
          };
          
          python = mkOption {
            type = types.submodule {
              options = {
                enable = mkOption {
                  type = types.bool;
                };

                package = mkOption {
                  type = types.package;
                };

                settings = mkOption {
                  type = types.submodule { freeformType = json; };
                };
              };
            };
          };

          js = mkOption {
            type = types.submodule {
              options = {
                enable = mkOption {
                  type = types.bool;
                };

                package = mkOption {
                  type = types.package;
                };

                settings = mkOption {
                  type = types.submodule { freeformType = json; };
                };
              };
            };
          };
        };
      };

      default = {
        enable = mkDefault true;
        package = mkDefault (import ../packages/sublime-debugger.nix { inherit pkgs; });
        settings = {};

        lldb = {
          enable = mkDefault true;
          package = mkDefault (import ../packages/codelldb.nix { inherit pkgs; });
          settings = {};
        };

        js = {
          enable = mkDefault true;
          package = mkDefault (import ../packages/vscode-js-debug.nix { inherit pkgs; });
          settings = {};
        };

        python = {
          enable = mkDefault true;
          package = mkDefault (import ../packages/vscode-python-debug.nix { inherit pkgs; });
          settings = {};
        };
      };
    };

    keymaps = mkOption {
      type = types.submodule { freeformType = json; };
      default = {
        "Default (OSX)" = [
          {
            keys = [ "shift" "shift" ];
            command = "show_overlay";
            args.overlay = "goto";
            args.show_files = true;
          }
          {
            keys = [ "super+r" ];
            command = "show_panel";
            args.panel = "replace";
            args.reverse = "false";
          }
          {
            keys = [ "option+enter" ];
            command = "lsp_hover";
            args = { };
          }
        ];
      };
    };

    schemes = mkOption {
      type = types.submodule { freeformType = json; };
      default = {
       
      };
    };

    themes = mkOption {
      type = types.submodule { freeformType = json; };
      default = {
        "Default Dark" = {
          rules = [ ];
          variables = {
            sidebar_bg = "rgb(43,45,48)";
            ui_bg = "rgb(43,45,48)";
            tabset_medium_dark_bg = "rgb(30,31,33)";
          };
        };
      };
    };

    settings = mkOption {
      type = types.submodule { freeformType = json; };
      default = {
        LSP = {
          clients = {
            nix = { 
              enabled = true; 
              command = ["${pkgs.nil}/bin/nil"]; 
              selector = "source.nix"; 
            };
          };
        };

        Preferences = {
          ignored_packages = [ "Vintage" ];
          theme = "auto";
          color_scheme = "Packages/User/Material-Theme-Darker.tmTheme";
          font_face = "";
          font_size = 13;
          save_on_focus_lost = true;
          find_selected_text = true;
        };

        "Package Control" = {
          bootstrapped = true;
          in_process_packages = [ ];
          installed_packages = (
            map (package: replaceStrings [".sublime-package"] [""] package.pname) config.lab.sublime.packages) ++ (
              if config.lab.sublime.debugger.enable then [ "SublimeDebugger" ] else [ ]
            );
        };

        JSON = {
          extensions = [
            "flake.lock"
            "package.lock"
            ".sublime-project"
          ];
        };
        
        language-ids = { 
          "source.nix" = "nix"; 
        };
      };
    };

    userFiles = mkOption {
      type = types.attrsOf types.anything;

      default = {
        "Material-Theme-Darker.tmTheme" = ../dotfiles/sublime/Material-Theme-Darker.tmTheme;
      };
    };
  };

  config.home = mkIf config.lab.sublime.enable {
    packages = [ config.lab.sublime.package pkgs.shellcheck ];

    file = 
      # Map package files
      (listToAttrs (map (package: { 
        name = package.pname; 
        value = {
          source = package;
          target = "${config.lab.sublime.packageDirectory}/${package.pname}";
        }; 
      }) 

        (config.lab.sublime.packages ++ (
          if config.lab.sublime.debugger.enable then
            [ config.lab.sublime.debugger.package ]
          else 
            []
          )
        ) 
      ))
      
      # Map user files
      // (mapAttrs (name: value: { 
        source = value;
        target = "${config.lab.sublime.userDirectory}/${name}";
      }) config.lab.sublime.userFiles)
      
      # Map settings files
      // (mkConfigFiles "sublime-keymap" config.lab.sublime.keymaps)
      // (mkConfigFiles "sublime-settings" config.lab.sublime.settings)
      // (mkConfigFiles "sublime-theme" config.lab.sublime.themes)
      // (mkConfigFiles "sublime-color-scheme" config.lab.sublime.schemes)

      # Map debugger files
      // (if config.lab.sublime.debugger.enable then {
        sublime-debugger-adapter-config = {
          source = ../dotfiles/sublime/debugger/sublime-package.json;
          target = "${config.lab.sublime.userDirectory}/../SublimeDebugger/sublime-package.json";
        };
      
        sublime-debugger-data-keep = {
          source = ../dotfiles/sublime/debugger/.gitkeep;
          target = "${config.lab.sublime.userDirectory}/../SublimeDebugger/data/.gitkeep";
        };
      
        sublime-debugger-config = {
          source = pkgs.writeTextFile {
            name = "Debugger.sublime-settings";
            text = toJSON config.lab.sublime.debugger.settings;
          };
      
          target = "${config.lab.sublime.userDirectory}/Debugger.sublime-settings";
        };
      } else {})

      # Map debugger adapters
      // (mkAdapter "lldb" config.lab.sublime.debugger.lldb)
      // (mkAdapter "python" config.lab.sublime.debugger.python)
      // (mkAdapter "js" config.lab.sublime.debugger.js);
  };
}
