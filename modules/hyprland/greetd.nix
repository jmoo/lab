{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule forLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      config = mkIf config.greetd.enable (
        forLinux (
          { pkgs, ... }:
          {
            services.greetd = {
              enable = true;
              settings.default_session = {
                command = "${pkgs.tuigreet}/bin/tuigreet --time --time-format '%I:%M %p | %a • %h | %F' --cmd 'uwsm start hyprland-uwsm.desktop'";
                user = "greeter";
              };
            };
          }
        )
      );

      options.greetd.enable = mkEnableOption "greetd nixos configuration";
    }
  );
}
