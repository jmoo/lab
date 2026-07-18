{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib') mkEnableOption mkIf mkMerge;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.claude = {
        enable = mkEnableOption "the claude-code program";
        skills.sensei.enable = mkEnableOption "the sensei Claude Code skill (Japanese study via anki-tool)";
      };

      config = mkMerge [
        (mkIf config.claude.enable {
          home.module.programs.claude-code.enable = true;
        })
        (mkIf config.claude.skills.sensei.enable {
          home.module =
            { pkgs, ... }:
            {
              home.packages = [ pkgs.anki-tool ];
              programs.claude-code.skills.sensei = ./sensei;
            };
        })
      ];
    }
  );
}
