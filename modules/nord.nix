{ lib', ... }:
let
  inherit (lib'.lab) forLinux mkHostModule;
  inherit (lib') mkEnableOption mkIf optionalString;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.nord.enable = mkEnableOption "host access to Clavia / Nord instruments over USB (nord-utils)";

      # A Nord is a composite device: the vendor-specific interface nord-usb talks to
      # sits alongside the audio and MIDI ones, which snd-usb-audio binds. Interfaces
      # bind independently, so the vendor interface is left unbound and needs no
      # detach — only permission to open the usbfs node, which is what this grants.
      #
      # uaccess alone is not enough on a machine reached over SSH: it is a logind ACL
      # for the active seat session, so it disappears whenever nobody is logged in
      # locally. The group fallback is what makes remote work reliable.
      config = mkIf config.nord.enable (
        forLinux (
          { config, ... }:
          {
            services.udev.extraRules = ''
              SUBSYSTEM=="usb", ATTR{idVendor}=="0ffc", TAG+="uaccess", GROUP="dialout", MODE="0660"
            ''
            # Sniffing our own traffic back off the bus, to diff against the captures
            # taken from Nord Sound Manager. Needs the usbmon module loaded to do
            # anything; the rule is only about who may read it.
            + optionalString config.programs.wireshark.enable ''
              SUBSYSTEM=="usbmon", GROUP="wireshark", MODE="0640"
            '';
          }
        )
      );
    }
  );
}
