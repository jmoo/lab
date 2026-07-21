{ lib', ... }:
let
  inherit (lib'.lab) homeLinux mkHostModule;
  inherit (lib')
    escapeShellArg
    mkEnableOption
    mkIf
    mkMerge
    mkOption
    types
    ;
    
  trackerPrompt = lib'.removeSuffix "\n" (builtins.readFile ./tracker-prompt.md);
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    let
      sensei = config.claude.skills.sensei;
    in
    {
      options.claude.skills.sensei = {
        enable = mkEnableOption "the sensei Claude Code skill (Japanese study via anki-tool)";

        tracker = {
          enable = mkEnableOption "daily data-only Anki progress tracker";

          onCalendar = mkOption {
            default = "*-*-* 00:00:00";
            description = "systemd OnCalendar expression for the tracker (local time)";
            type = types.str;
          };

          vault = mkOption {
            default = "/home/jmoore/Repos/jmoo/notes";
            description = "Notes vault the tracker logs into";
            type = types.str;
          };
        };
      };

      config = mkMerge [
        (mkIf sensei.enable {
          home.module =
            { pkgs, ... }:
            {
              home.packages = [ pkgs.anki-tool ];
              programs.claude-code.skills.sensei = ./skill;
            };
        })
        (mkIf (sensei.enable && sensei.tracker.enable) (
          homeLinux (
            { config, pkgs, ... }:
            let
              sensei-tracker = pkgs.writeShellApplication {
                name = "sensei-tracker";
                runtimeInputs = [
                  config.programs.claude-code.package
                  pkgs.anki-tool
                  pkgs.git
                ];
                text = ''
                  cd ${escapeShellArg sensei.tracker.vault}

                  # Cheap guard: do nothing (and spend no Claude invocation) when
                  # Anki isn't running.
                  if ! anki-tool overview >/dev/null 2>&1; then
                    echo "sensei-tracker: AnkiConnect unreachable (Anki closed?); skipping"
                    exit 0
                  fi

                  # Permissions come from the vault's own .claude/settings.json
                  # (anki-tool, git add/commit/push/pull, Edit, Write, Skill(sensei)),
                  # loaded because we cd'd in above — no --dangerously-skip-permissions.
                  claude -p ${escapeShellArg trackerPrompt}
                '';
              };
            in
            {
              home.packages = [ sensei-tracker ];

              systemd.user = {
                services.sensei-tracker = {
                  Service = {
                    ExecStart = "${config.home.profileDirectory}/bin/sensei-tracker";
                    Type = "oneshot";
                  };
                  Unit.Description = "Daily Anki progress tracker";
                };

                timers.sensei-tracker = {
                  Install.WantedBy = [ "timers.target" ];
                  Timer = {
                    # No Persistent: a catch-up run after 4am would read the wrong
                    # Anki day (see prompt's date nuance).
                    OnCalendar = sensei.tracker.onCalendar;
                    Persistent = false;
                  };
                  Unit.Description = "Daily Anki progress tracker timer";
                };
              };
            }
          )
        ))
      ];
    }
  );
}
