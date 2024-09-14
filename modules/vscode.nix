{ pkgs, lib, config, ... }:

with builtins;
with lib;

{
  options.lab.vscode = { enable = mkEnableOption "vscode"; };

  config.home.packages = with pkgs;
    mkIf config.lab.vscode.enable [ nil nixfmt shellcheck direnv ];

  config.programs.vscode = mkIf config.lab.vscode.enable {
    enable = true;

    extensions = with pkgs.vscode-extensions;
      [
        # nix-vscode-extend
        # mkhl.direnv
        # rust-lang.rust-analyzer
        # ms-python.python
        jnoortheen.nix-ide
        # ms-vscode-remote.remote-ssh
        # ms-azuretools.vscode-docker
        # usernamehw.errorlens
        # dbaeumer.vscode-eslint
        # esbenp.prettier-vscode
        # streetsidesoftware.code-spell-checker
        # tamasfe.even-better-toml
        # mads-hartmann.bash-ide-vscode
      ];

    nixExtensions.default = {
      paths = [{
        from = ../resources/icons;
        to = "./nix/store/icons";
      }];

      themes = {
        jmoo-dark = {
          path = ../resources/jmoo-dark.json;
          uiTheme = "vs-dark";
        };
      };

      iconThemes = {
        jmoo-dark-icons.path = ../resources/jmoo-dark-icons.json;
      };

      commands = {
        testGenExe.exec =
          pkgs.writeShellScript "say-hello.sh" "echo hello from an exe";
        testGenScript.exec = "echo hello from a script";
        testGenJs.require =
          pkgs.writeText "say-hello.js" "console.log('hello from node')";
        testGenCommandReload.commands = [ "workbench.action.reloadWindow" ];
      };
    };

    keybindings = [
      {
        key = "ctrl+cmd+p";
        command = "workbench.action.openRecent";
      }

      {
        key = "alt+enter";
        command = "editor.action.quickFix";
        when =
          "editorHasCodeActionsProvider && textInputFocus && !editorReadonly";
      }

      {
        key = "alt+cmd+l";
        command = "editor.action.formatDocument";
        when =
          "editorHasDocumentFormattingProvider && editorTextFocus && !editorReadonly && !inCompositeEditor";
      }

      {
        key = "cmd+r";
        command = "editor.action.startFindReplaceAction";
        when = "editorFocus || editorIsOpen";
      }
    ];

    userSettings = {
      extensions.autoUpdate = false;
      git.openRepositoryInParentFolders = "always";
      "workbench.activityBar.location" = "top";
      "workbench.sideBar.location" = "left";
      terminal.integrated.fontFamily = "MesloLGS NF";
      terminal.integrated.defaultProfile.osx = "zsh";
      terminal.external.osxExec = "iTerm.app";
      terminal.integrated.fontSize = 13;

      nix.enableLanguageServer = true;
      nix.serverPath = "nil";
      nix.serverSettings = { nil.formatting.command = [ "nixfmt" ]; };

      files.associations = { "*.json" = "jsonc"; };

      "editor.fontFamily" = "Ubuntu Mono";
      "editor.fontSize" = 14;
      "editor.lineHeight" = 1.2;
      "workbench.colorTheme" = "jmoo-dark";
      "workbench.iconTheme" = "jmoo-dark-icons";
      "editor.semanticHighlighting.enabled" = true;
      "workbench.tree.indent" = 20;
      "editor.semanticTokenColorCustomizations" = {
        enabled = true;
        rules = { "*.mutable" = { "underline" = false; }; };
      };
    };
  };
}
