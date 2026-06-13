{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      config = mkIf config.iterm2.enable {
        darwin = {
          home =
            { pkgs, ... }:
            {
              home = {
                file.iterm2-plist = {
                  executable = false;
                  source = ./iterm2.plist;
                  target = ".config/iterm2/com.googlecode.iterm2.plist";
                };

                packages = [ pkgs.iterm2 ];
              };
            };

          module =
            { pkgs, ... }:
            {
              environment.systemPackages = [ pkgs.iterm2 ];
            };
        };
      };

      options.iterm2.enable = mkEnableOption "iterm2 home-manager configuration";
    }
  );
}
