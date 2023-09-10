sudo systemd-run \
    --service-type=notify \
    --description="Tailscale node agent" \
    -u tailscaled.service \
    -p ExecStartPre="/home/deck/.nix-profile/bin/tailscaled --cleanup" \
    -p ExecStopPost="/home/deck/.nix-profile/bin/tailscaled --cleanup" \
    -p Restart=on-failure \
    -p RuntimeDirectory=tailscale \
    -p RuntimeDirectoryMode=0755 \
    -p StateDirectory=tailscale \
    -p StateDirectoryMode=0700 \
    -p CacheDirectory=tailscale \
    -p CacheDirectoryMode=0750 \
    "/home/deck/.nix-profile/bin/tailscaled" \
    "--state=/var/lib/tailscale/tailscaled.state" \
    "--socket=/run/tailscale/tailscaled.sock"
