{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule forLinux homeDarwin;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.mosh.enable = mkEnableOption "mosh";

      config = mkIf config.mosh.enable (
        forLinux (_: {
          programs.mosh.enable = true;
        })
        // homeDarwin (
          { pkgs, ... }:
          {
            home.packages = [ pkgs.mosh ];
          }
        )
      );
    }
  );
}
