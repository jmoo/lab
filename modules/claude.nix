{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.claude.tutor.enable = mkEnableOption "the anki-tutor Claude Code skill (Japanese study via anki-tool)";

      config = mkIf config.claude.tutor.enable {
        home.module =
          { pkgs, ... }:
          {
            home.packages = [ pkgs.anki-tool ];
            programs.claude-code = {
              enable = true;
              skills.anki-tutor = ./claude/anki-tutor;
            };
          };
      };
    }
  );
}
