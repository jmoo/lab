{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.iterm2.enable = mkEnableOption "iterm2 home-manager configuration";

      config = mkIf config.iterm2.enable {
        darwin = {
          module =
            { pkgs, ... }:
            {
              environment.systemPackages = [ pkgs.iterm2 ];
            };

          home =
            { pkgs, ... }:
            {
              home = {
                packages = [ pkgs.iterm2 ];

                file.iterm2-plist = {
                  executable = false;
                  source = ./iterm2.plist;
                  target = ".config/iterm2/com.googlecode.iterm2.plist";
                };
              };
            };
        };
      };
    }
  );
}
