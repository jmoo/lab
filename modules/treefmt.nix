{ inputs, ... }:
{
  imports = [ inputs.treefmt-nix.flakeModule ];

  perSystem.treefmt = {
    programs = {
      nixfmt.enable = true;
      rustfmt = {
        # treefmt invokes rustfmt on bare paths, so it never sees a Cargo.toml
        # and never picks up the workspace edition — leaving this unset formats
        # the crates as 2024.
        edition = "2021";
        enable = true;
      };
      shfmt.enable = true;
      taplo.enable = true;
    };

    projectRootFile = "flake.nix";
  };
}
