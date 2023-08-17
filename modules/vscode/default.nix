{ pkgs, lib, config, ... }:

with builtins;
with lib;

let
  customExtension = (import ./extension/default.nix { inherit pkgs; });
in
{
  options.lab.vscode = { enable = mkEnableOption "vscode"; };

  config.home.packages = with pkgs;
    mkIf config.lab.vscode.enable [ nil nixfmt shellcheck direnv ];

  config.programs.vscode = mkIf config.lab.vscode.enable {
    enable = true;

    extensions = with pkgs.vscode-extensions;
      [
        customExtension
        mkhl.direnv
        rust-lang.rust-analyzer
        ms-python.python
        jnoortheen.nix-ide
        ms-vscode-remote.remote-ssh
        mads-hartmann.bash-ide-vscode
        ms-azuretools.vscode-docker
        usernamehw.errorlens
      ] ++ (if pkgs.stdenv.isAarch64 then [ ] else [ 
        vadimcn.vscode-lldb
        ms-vscode.cpptools 
      ]);

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
      git.openRepositoryInParentFolders = "always";
      workbench.activityBar.visible = false;

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
