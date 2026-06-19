{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.direnv.enable = mkEnableOption "direnv home-manager configuration";

      config = mkIf config.direnv.enable {
        home.module =
          { pkgs, ... }:
          {
            programs = {
              direnv = {
                enable = true;
                enableBashIntegration = false;
                enableFishIntegration = false;
                enableNushellIntegration = false;
                enableZshIntegration = false;
                nix-direnv.enable = false;
              };

              vscode.profiles.default.extensions = with pkgs.vscode-extensions; [ mkhl.direnv ];
            };
          };
      };
    }
  );
}
