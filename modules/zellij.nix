{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.zellij.enable = mkEnableOption "zellij terminal multiplexer";

      config = mkIf config.zellij.enable {
        home.module = _: {
          programs.zellij = {
            enable = true;
            enableZshIntegration = true;
          };
        };
      };
    }
  );
}
