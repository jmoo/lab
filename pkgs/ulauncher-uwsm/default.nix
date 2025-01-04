{ ulauncher, ... }:
ulauncher.overrideAttrs (old: {
  patches = old.patches ++ [
    ./uwsm-launcher-v5.patch
  ];
})
