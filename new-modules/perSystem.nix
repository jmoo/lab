{ inputs, ... }:
{
  systems = [
    "x86_64-linux"
    "aarch64-linux"
    "aarch64-darwin"
  ];

  flake.overlays.default = import ../overlay.nix inputs;

  perSystem =
    { system, ... }:
    let
      pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [ (import ../overlay.nix inputs) ];
        # Match the per-platform allowUnfree policy: not on aarch64-linux (Asahi).
        config.allowUnfree = system != "aarch64-linux";
      };
    in
    {
      _module.args.pkgs = pkgs;
      formatter = pkgs.nixfmt-tree;
      legacyPackages = pkgs;
    };
}
