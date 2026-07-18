{ ... }:
{
  perSystem =
    { pkgs, ... }:
    {
      # Dev shell for iterating on the Rust workspace under crates/ outside Nix.
      # `nix develop` then `cd crates && cargo test`. mkShell's stdenv supplies
      # the C toolchain (cc/linker) cargo needs.
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          clippy
          rust-analyzer
          rustc
          rustfmt
        ];
      };
    };
}
