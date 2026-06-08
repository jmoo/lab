{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeAll;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.direnv.enable = mkEnableOption "direnv home-manager configuration";

      config = mkIf config.direnv.enable (
        homeAll (
          { pkgs, ... }:
          {
            programs = {
              direnv = {
                enable = true;
                enableZshIntegration = false;
                enableBashIntegration = false;
                enableNushellIntegration = false;
                enableFishIntegration = false;
                nix-direnv.enable = false;
              };

              vscode.profiles.default.extensions = with pkgs.vscode-extensions; [ mkhl.direnv ];
            };
          }
        )
      );
    }
  );
}
