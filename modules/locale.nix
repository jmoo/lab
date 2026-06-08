{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule forLinux;
in
{
  # Locale + timezone, applied to every Linux platform (nix-darwin has no
  # i18n/time.timeZone NixOS options).
  options.lab.hosts = mkHostModule (_: {
    config = forLinux {
      i18n = {
        defaultLocale = "en_US.UTF-8";
        extraLocaleSettings = {
          LC_ADDRESS = "en_US.UTF-8";
          LC_IDENTIFICATION = "en_US.UTF-8";
          LC_MEASUREMENT = "en_US.UTF-8";
          LC_MONETARY = "en_US.UTF-8";
          LC_NAME = "en_US.UTF-8";
          LC_NUMERIC = "en_US.UTF-8";
          LC_PAPER = "en_US.UTF-8";
          LC_TELEPHONE = "en_US.UTF-8";
          LC_TIME = "en_US.UTF-8";
        };
      };

      time.timeZone = "America/New_York";
    };
  });
}
