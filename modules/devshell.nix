{ ... }:
{
  perSystem =
    { pkgs, ... }:
    {
      # Dev shell for iterating on the Rust workspace under crates/ outside Nix.
      # `nix develop` then `cd crates && cargo test`. mkShell's stdenv supplies
      # the C toolchain (cc/linker) cargo needs. `lld` is a separate matter: it is
      # the linker rustc invokes for wasm32-unknown-unknown, and nixpkgs' rustc
      # does not carry its own copy, so without it a wasm cdylib fails to link
      # with "linker `lld` not found".
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          clippy
          lld
          rust-analyzer
          rustc
          rustfmt
        ];
      };
    };
}
