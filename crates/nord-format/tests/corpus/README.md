# Curated specimen corpus

A small, committed subset of the Electro 5 change-one-knob specimen corpus, laid
out as the round-trip tests in `../ne5.rs` expect:

```
song.ne5t
settings.ne5s
programs/center_panel/*.ne5p
programs/gain/*.ne5p
programs/fx/*.ne5p
programs/equalizer/*.ne5p
programs/sample/*.ne5p
programs/organ/*.ne5p
```

Filenames encode the parameter values under test (conventions documented per
panel in the `nord-utils` workbench). The tests **skip** any panel whose
directory is empty/absent, so the suite stays green even before this is
populated.

The **full** ~350-file specimen set lives in the `nord-utils` RE workbench
(kept out of this workspace so it doesn't bloat every crate's build hash). Run
the exhaustive sweep by pointing `NORD_CORPUS_DIR` at it:

```sh
NORD_CORPUS_DIR=~/Repos/jmoo/nord-utils/archive/resources/ne5 cargo test -p nord-format
```
