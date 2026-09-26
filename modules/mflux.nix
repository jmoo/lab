{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeDarwin;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.mflux.enable = mkEnableOption "mflux image generation on Apple Silicon (darwin)";

      config = mkIf config.mflux.enable (
        homeDarwin (
          { pkgs, ... }:
          {
            home.packages = [ pkgs.python3Packages.mflux ];
          }
        )
      );
    }
  );
}
