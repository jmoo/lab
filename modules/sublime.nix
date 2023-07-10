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
        (import ../packages/sublime-nix.nix { inherit pkgs; })
        (import ../packages/sublime-copilot.nix { inherit pkgs; })
        (import ../packages/sublime-rust-analyzer.nix { inherit pkgs; })
        (import ../packages/sublime-typescript.nix { inherit pkgs; })
        (import ../packages/sublime-toml.nix { inherit pkgs; })
        (import ../packages/sublime-sidebar-enhancements.nix { inherit pkgs; })
      ];
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
            nix = { enabled = true; command = ["nil"]; selector = "source.nix"; };
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
          installed_packages = map (package: replaceStrings [".sublime-package"] [""] package.pname) config.lab.sublime.packages;
        };

        language-ids = { 
          "source.nix" = "nix"; 
          "flake.lock" = "JSON";
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
    packages = [ config.lab.sublime.package ];

    file = 
      # Map package files
      (listToAttrs (map (package: { 
        name = package.pname; 
        value = {
          source = package;
          target = "${config.lab.sublime.packageDirectory}/${package.pname}";
        }; 
      }) config.lab.sublime.packages))
      
      # Map user files
      // (mapAttrs (name: value: { 
        source = value;
        target = "${config.lab.sublime.userDirectory}/${name}";
      }) config.lab.sublime.userFiles)
      
      # Map settings files
      // (mkConfigFiles "sublime-keymap" config.lab.sublime.keymaps)
      // (mkConfigFiles "sublime-settings" config.lab.sublime.settings)
      // (mkConfigFiles "sublime-theme" config.lab.sublime.themes)
      // (mkConfigFiles "sublime-color-scheme" config.lab.sublime.schemes);
  };
}
