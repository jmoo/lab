{ lib', ... }:
let
  inherit (lib'.lab)
    forLinux
    homeLinux
    mkHostModule
    ;
  inherit (lib') mkEnableOption mkIf mkMerge;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.orca-slicer.enable = mkEnableOption "Orca Slicer with the open-bamboo-networking plugin (LAN printing for Bambu printers)";

      # Bambu's proprietary network plugin abort()s under NixOS' runtime
      # (libstdc++ allocator mismatch -> "free(): invalid size"). We provision
      # the open-source replacement instead: autoPatchelf'd .so files sharing
      # the slicer's exact glibc/libstdc++, synced into the slicer's mutable
      # config dir on activation (the slicer owns these paths at runtime, so a
      # store symlink is not an option) and the plugin version pinned in
      # OrcaSlicer.conf so the slicer dlopens our build. LAN printing additionally
      # needs the printer in Developer Mode (firmware MQTT command signing).
      config = mkIf config.orca-slicer.enable (mkMerge [
        (forLinux (
          { pkgs, ... }:
          {
            environment.systemPackages = [ pkgs.orca-slicer ];

            # LAN discovery: Bambu printers announce over multicast + UDP 1990/2021.
            networking.firewall.extraCommands = ''
              iptables -I INPUT -m pkttype --pkt-type multicast -j ACCEPT
              iptables -I INPUT -p udp -m udp --match multiport --dports 1990,2021 -j ACCEPT
            '';
          }
        ))
        (homeLinux (
          {
            lib,
            pkgs,
            ...
          }:
          let
            obn = pkgs.open-bamboo-networking;
          in
          {
            home.activation.orcaSlicerBambuPlugin = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
              orcaDir="''${XDG_CONFIG_HOME:-$HOME/.config}/OrcaSlicer"
              run mkdir -p "$orcaDir/plugins"
              run install -m644 ${obn}/lib/libbambu_networking_${obn.pluginVersion}.so \
                "$orcaDir/plugins/libbambu_networking_${obn.pluginVersion}.so"
              run install -m644 ${obn}/lib/libBambuSource.so "$orcaDir/plugins/libBambuSource.so"
              run install -m644 ${obn}/lib/liblive555.so "$orcaDir/plugins/liblive555.so"

              conf="$orcaDir/OrcaSlicer.conf"
              if [ -f "$conf" ]; then
                ${pkgs.jq}/bin/jq --arg v "${obn.pluginVersion}" \
                  '.app.network_plugin_version = $v
                   | .app.installed_networking = true
                   | .app.network_plugin_remind_later = true' \
                  "$conf" > "$conf.obn-tmp" \
                  && run mv "$conf.obn-tmp" "$conf"
              fi
            '';
          }
        ))
      ]);
    }
  );
}
