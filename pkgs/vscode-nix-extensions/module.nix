{ lib, config, ... }:
with lib;
{
  options = {
    name = mkOption { type = types.str; };

    version = mkOption {
      type = types.str;
      default = "0.0.0";
    };

    publisher = mkOption {
      type = types.str;
      default = "vscode-nix-extensions-generator";
    };

    paths = mkOption {
      type =
        with types;
        listOf (oneOf [
          package
          pathInStore
          (submodule {
            options = {
              from = mkOption {
                type = oneOf [
                  package
                  pathInStore
                ];
              };
              to = mkOption { type = str; };
            };
          })
        ]);
      default = [ ];
      description = "Nix store paths to link into the extension directory";
    };

    displayName = mkOption { type = types.str; };

    debug = mkEnableOption "Enable vscode extension debugging.";

    description = mkOption {
      type = types.lines;
      default = "";
    };

    extraConfig = mkOption {
      type = types.submodule {
        _module.freeformType = with types; attrsOf anything;
      };
      default = { };
    };

    iconThemes = mkOption {
      type =
        with types;
        attrsOf (
          submodule (
            { name, ... }:
            {
              options = {
                id = mkOption {
                  type = with types; str;
                  default = name;
                };

                label = mkOption {
                  type = with types; str;
                  description = "Name of the theme";
                  default = name;
                };

                path = mkOption {
                  type =
                    with types;
                    nullOr (oneOf [
                      package
                      pathInStore
                    ]);
                  description = "Theme file";
                };
              };
            }
          )
        );
      default = { };
    };

    themes = mkOption {
      type =
        with types;
        attrsOf (
          submodule (
            { name, ... }:
            {
              options = {
                id = mkOption {
                  type = with types; str;
                  default = name;
                };

                label = mkOption {
                  type = with types; str;
                  description = "Name of the theme";
                  default = name;
                };

                path = mkOption {
                  type =
                    with types;
                    nullOr (oneOf [
                      package
                      pathInStore
                    ]);
                  description = "Theme file";
                };

                uiTheme = mkOption {
                  type =
                    with types;
                    enum [
                      "vs-light"
                      "vs-dark"
                    ];
                };
              };
            }
          )
        );
      default = { };
    };

    commands = mkOption {
      type =
        with types;
        attrsOf (
          submodule (
            { name, ... }@self:
            {
              options = {
                enable = mkOption {
                  type = types.bool;
                  default = true;
                  description = "Enable the command";
                };

                title = mkOption {
                  type = types.str;
                  description = "Human readable name for the command";
                };

                id = mkOption {
                  type = types.str;
                  description = "Full command identifier";
                };

                exec = mkOption {
                  type =
                    with types;
                    nullOr (oneOf [
                      str
                      package
                    ]);
                  default = null;
                  description = "Executable or shell command to execute";
                };

                require = mkOption {
                  type =
                    with types;
                    nullOr (oneOf [
                      str
                      package
                    ]);
                  default = null;
                  description = "JS Script to execute";
                };

                commands = mkOption {
                  type = with types; listOf str;
                  default = [ ];
                  description = "Commands from other extensions to execute";
                };
              };

              config = {
                id = mkOptionDefault "${config.name}.${name}";
                title = mkOptionDefault self.name;
              };
            }
          )
        );
      default = { };
    };
  };

  config = {
    displayName = mkOptionDefault config.name;

    paths = foldl' (acc: x: acc ++ [ x.path ]) [ ] (
      (attrValues config.themes) ++ (attrValues config.iconThemes)
    );

    extraConfig = {
      inherit (config)
        name
        version
        description
        publisher
        ;
      engines.vscode = "^1.81.1";
      categories = [ ];
      main = "./extension.js";
      nixExtension = {
        inherit (config) commands;
      };
      contributes = {
        commands = mapAttrsToList (_: cmd: {
          inherit (cmd) title;
          command = cmd.id;
        }) config.commands;

        themes = map (x: x // { path = ".${x.path}"; }) (attrValues config.themes);

        iconThemes = map (x: x // { path = ".${x.path}"; }) (attrValues config.iconThemes);
      };
    };
  };
}
