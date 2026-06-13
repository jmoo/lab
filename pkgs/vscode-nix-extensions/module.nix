{ lib, config, ... }:
with lib;
{
  options = {
    commands = mkOption {
      type =
        with types;
        attrsOf (
          submodule (
            { name, ... }@self:
            {
              config = {
                id = mkOptionDefault "${config.name}.${name}";
                title = mkOptionDefault self.name;
              };

              options = {
                commands = mkOption {
                  type = with types; listOf str;
                  default = [ ];
                  description = "Commands from other extensions to execute";
                };

                enable = mkOption {
                  type = types.bool;
                  default = true;
                  description = "Enable the command";
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

                id = mkOption {
                  type = types.str;
                  description = "Full command identifier";
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

                title = mkOption {
                  type = types.str;
                  description = "Human readable name for the command";
                };
              };
            }
          )
        );
      default = { };
    };

    debug = mkEnableOption "Enable vscode extension debugging.";

    description = mkOption {
      type = types.lines;
      default = "";
    };

    displayName = mkOption { type = types.str; };

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
                  default = name;
                  description = "Name of the theme";
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

    name = mkOption { type = types.str; };

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

    publisher = mkOption {
      type = types.str;
      default = "vscode-nix-extensions-generator";
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
                  default = name;
                  description = "Name of the theme";
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

    version = mkOption {
      type = types.str;
      default = "0.0.0";
    };
  };

  config = {
    displayName = mkOptionDefault config.name;

    extraConfig = {
      categories = [ ];
      contributes = {
        commands = mapAttrsToList (_: cmd: {
          inherit (cmd) title;
          command = cmd.id;
        }) config.commands;

        iconThemes = map (x: x // { path = ".${x.path}"; }) (attrValues config.iconThemes);

        themes = map (x: x // { path = ".${x.path}"; }) (attrValues config.themes);
      };
      inherit (config)
        description
        name
        publisher
        version
        ;
      engines.vscode = "^1.81.1";
      main = "./extension.js";
      nixExtension = {
        inherit (config) commands;
      };
    };

    paths = foldl' (acc: x: acc ++ [ x.path ]) [ ] (
      (attrValues config.themes) ++ (attrValues config.iconThemes)
    );
  };
}
