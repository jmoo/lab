# nord-cli

A thin command-line tool (`nord`) over the [`nord-format`](../nord-format)
library. Its job is to **dogfood the library**, not to be a product.

```sh
# Summary of a program, song, settings, piano, sample, or backup bundle:
nord inspect path/to/patch.ne5p

# Several at once:
nord inspect *.ne5p

# Raw decoded structure (full Debug dump):
nord inspect --raw path/to/song.ne5t
```

Build/run from the lab workspace:

```sh
nix develop
cd crates
cargo run -p nord-cli -- inspect ~/Repos/jmoo/nord-utils/archive/resources/ne5/song.ne5t
```

Or as a Nix package: `nix build .#nord-cli` (installs the `nord` binary).
