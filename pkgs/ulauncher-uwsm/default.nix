{ ulauncher, ... }:
ulauncher.overrideAttrs (old: {
  patches = [
    ./uwsm-launcher-v5.patch
  ];
})
