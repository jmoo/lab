# Vscode Nix Extensions

Generate vscode extensions from nix expressions.


```nix
# Home manager example
programs.vscode = {
  nixExtensions.default = {
    # Add themes
    themes = {
      foo = {
        path = ./path/to/theme.json;
        uiTheme = "vs-dark";
      };
    };

    # Define commands
    commands.sayHello.exec = "echo hi there";
    commands.doManyThings.commands = [
       "some.commandId"
       "another.commandId"
    ];
  };
}

```