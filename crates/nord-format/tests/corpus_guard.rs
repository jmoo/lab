//! The one test target that compiles under every feature set.
//!
//! ⚠️ The corpus suites (`ne5.rs`, `decode_snapshot.rs`, `mutation.rs`) are
//! `#![cfg(feature = "corpus")]`, so without the feature they compile to nothing and
//! `cargo test` reports a pass having verified none of the decode. A set
//! `NORD_CORPUS_DIR` is someone saying they meant to run them.

/// `NORD_CORPUS_DIR` set with the feature off means the corpus suite did not run.
#[test]
fn corpus_dir_without_the_corpus_feature_is_a_mistake() {
    #[cfg(not(feature = "corpus"))]
    assert!(
        std::env::var_os("NORD_CORPUS_DIR").is_none(),
        "NORD_CORPUS_DIR is set but --features corpus is off: the corpus suite compiled \
         out and this run verified none of it. The full command is\n    \
         cargo test --workspace --features nord-usb/replay,nord-format/corpus"
    );
}
