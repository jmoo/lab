{ ... }:
{
  perSystem =
    { lib, pkgs, ... }:
    let
      # nord-gui links its window and GL bindings at runtime via dlopen, so they have
      # to be findable rather than merely built against. The packaged crate wraps
      # itself (see overlay.nix); `cargo run` inside the shell needs the same.
      guiLibraries = lib.makeLibraryPath (
        with pkgs;
        [
          libGL
          libxkbcommon
          wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ]
      );
    in
    {
      # Dev shell for iterating on the Rust workspace under crates/ outside Nix.
      # `nix develop` then `cd crates && cargo test`. mkShell's stdenv supplies
      # the C toolchain (cc/linker) cargo needs.
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          clippy
          # wasm32 needs a linker rustc does not bring itself; `trunk` drives the
          # nord-gui browser build (`cd crates/nord-gui && trunk serve`).
          lld
          rust-analyzer
          rustc
          rustfmt
          trunk
        ];

        shellHook = ''
          export LD_LIBRARY_PATH=${guiLibraries}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
        '';
      };
    };
}
