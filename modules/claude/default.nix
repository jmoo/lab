{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.claude.enable = mkEnableOption "the claude-code program";

      config = mkIf config.claude.enable {
        home.module.programs.claude-code.enable = true;
      };
    }
  );
}
