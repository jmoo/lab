//! The captured exchange shapes, checked against what this crate puts on the wire.
//!
//! The corpus commits 34 `usb/**/*.shape.txt` files: for each NSM capture, the
//! **payload size of every frame in order**, run-length encoded as `O<bytes>` /
//! `I<bytes>`. Sizes, not bytes — so what is checkable here is framing and structure,
//! and nothing about argument *values*. Values are covered byte-exactly by the golden
//! replays in `ops.rs` and `status.rs`, for the six captures whose bytes the corpus
//! archives; this harness covers 30 more captures at the level the material supports.
//!
//! Three checks, in increasing strength:
//!
//! * `shape` — the file's own arithmetic. Its header states per-endpoint URB and byte
//!   totals, which the run-length lines must add up to. This checks the reader in this
//!   file, not the crate, and exists so a misparse cannot quietly weaken the two below.
//! * `grammar` — every frame is a legal size for [`wire`]: a request is at least a
//!   header plus a CRC, a response additionally carries a status word, and the device
//!   never speaks unbidden, so a capture never holds more INs than OUTs. Runs on the
//!   captures whose endpoint list is the vendor pair alone; a capture that also carries
//!   MIDI or control traffic has frames the vendor grammar does not describe and the
//!   file does not say which line is which.
//! * `transaction` — the strong one. Drive the operations NSM performed through this
//!   crate's own [`Session`] and [`op`] primitives against a responder that answers but
//!   asserts nothing, and require the sizes the host emitted to equal the capture's OUT
//!   sequence exactly, in order.
//!
//! ⚠️ **What `transaction` deliberately does not check.** Only the host half. A device
//! response size is the device's to choose and cannot be predicted from this crate
//! without implementing the instrument, so the IN sizes are used by `grammar` and by
//! nothing else. And because bank/slot arguments are fixed-width, a wrong address does
//! not change a size — that is the golden replays' job. What it does catch is an
//! argument list that gained or lost a field, an operation that stopped sending its
//! progress label, a session wrapper that changed shape, and a rename whose
//! length-prefixed string is encoded differently.
//!
//! Not reproduced, and why:
//!
//! * `relink_*` — command `0x35`'s payload semantics are not pinned down,
//!   so the crate builds no typed operation on it and there is nothing to drive.
//! * the read, write and bundle transfers, the full backup, the firmware update and the
//!   NSM startup sync — these are several transactions with NSM's host-side browser
//!   bookkeeping interleaved, which [`op`] documents as deliberately not reproduced.
//!   They are still covered by `grammar`.
//!
//! ```sh
//! NORD_CORPUS_DIR=/path/to/nord-corpus \
//!   cargo test -p nord-usb --features corpus --test corpus_shapes
//! ```

#[cfg(feature = "corpus")]
mod common;

#[cfg(feature = "corpus")]
mod shapes {
    use super::common::block_on;
    use libtest_mimic::{Arguments, Trial};
    use nord_usb::error::{Error, Result};
    use nord_usb::transport::Transport;
    use nord_usb::wire::{cmd, Location, Message, ObjectClass, Service, CRC_LEN, HEADER_LEN};
    use nord_usb::{envelope, op, Session};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    // -----------------------------------------------------------------------
    // The harness
    // -----------------------------------------------------------------------

    pub fn run() -> ! {
        let args = Arguments::from_args();
        let root = ne5_dir();
        let mut trials = Vec::new();

        for path in shape_files(&root) {
            let name = capture_name(&root, &path);
            let at = path.clone();
            trials.push(aggregate(&format!("shape/{name}"), move || {
                Shape::read(&at).check_against_its_own_totals()
            }));

            let at = path.clone();
            let label = name.clone();
            trials.push(aggregate(&format!("grammar/{name}"), move || {
                let shape = Shape::read(&at);
                if !shape.is_vendor_only() {
                    // Not a skip: the file mixes endpoints the vendor grammar does not
                    // describe and does not say which line came from which, so there is
                    // nothing here to assert. Named so the absence is visible.
                    println!(
                        "{label}: mixed endpoints {:?}, grammar not applicable",
                        shape.endpoints()
                    );
                    return;
                }
                shape.check_grammar();
            }));
        }

        for &(capture, class, ops) in TRANSACTIONS {
            let at = root.join("usb").join(capture);
            trials.push(aggregate(
                &format!("transaction/usb/{capture}"),
                move || Shape::read(&shape_file(&at)).check_transaction(class, ops),
            ));
        }

        for path in bodies(&root) {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            trials.push(aggregate(&format!("body/{name}"), move || {
                check_set_list_body(&path)
            }));
        }

        trials.push(aggregate("coverage/captures", {
            let root = root.clone();
            move || check_every_capture_is_accounted_for(&root)
        }));
        trials.push(aggregate("coverage/bodies", move || {
            let found = bodies(&root).len();
            assert!(found >= 3, "found only {found} set-list bodies");
        }));

        libtest_mimic::run(&args, trials).exit()
    }

    fn aggregate(name: &str, check: impl FnOnce() + Send + 'static) -> Trial {
        Trial::test(name.to_string(), move || {
            check();
            Ok(())
        })
    }

    /// The Electro 5 model directory of the specimen corpus.
    ///
    /// `NORD_CORPUS_DIR` names the corpus root — the directory holding every model —
    /// and the captures are the Electro 5's, so this joins its way in. Since this target
    /// only compiles under the `corpus` feature, a missing variable is a hard error.
    fn ne5_dir() -> PathBuf {
        let dir: PathBuf = std::env::var_os("NORD_CORPUS_DIR")
            .expect("set NORD_CORPUS_DIR to a nord-corpus checkout root for --features corpus")
            .into();
        dir.join("ne5")
    }

    fn shape_files(root: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![root.join("usb")];
        while let Some(dir) = stack.pop() {
            for entry in
                std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.to_string_lossy().ends_with(".shape.txt") {
                    found.push(path);
                }
            }
        }
        found.sort();
        assert!(
            found.len() >= 30,
            "found only {} shape files under {}/usb — is this the ne5 tree of a nord-corpus checkout?",
            found.len(),
            root.display(),
        );
        found
    }

    /// The one shape file in a capture directory.
    fn shape_file(dir: &Path) -> PathBuf {
        let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|e| e.unwrap().path())
            .filter(|p| p.to_string_lossy().ends_with(".shape.txt"))
            .collect();
        found.sort();
        assert_eq!(
            found.len(),
            1,
            "{} holds {} shape files, expected one",
            dir.display(),
            found.len(),
        );
        found.pop().unwrap()
    }

    /// The capture's directory, relative to the corpus root — what a reader would `cd`
    /// to.
    fn capture_name(root: &Path, shape: &Path) -> String {
        shape
            .parent()
            .unwrap()
            .strip_prefix(root)
            .unwrap_or(shape)
            .to_string_lossy()
            .into_owned()
    }

    fn bodies(root: &Path) -> Vec<PathBuf> {
        let dir = root.join("set_lists");
        let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|e| e.unwrap().path())
            .filter(|p| p.to_string_lossy().ends_with(".ne5t.body"))
            .collect();
        found.sort();
        found
    }

    // -----------------------------------------------------------------------
    // The shape file
    // -----------------------------------------------------------------------

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Dir {
        In,
        Out,
    }

    /// One capture's exchange shape.
    struct Shape {
        name: String,
        /// Every frame in order, run-length expanded.
        frames: Vec<(Dir, usize)>,
        /// `endpoint -> (urbs, bytes)`, as the file's header states them.
        endpoints: BTreeMap<String, (usize, usize)>,
    }

    impl Shape {
        fn read(path: &Path) -> Shape {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text =
                std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));

            let mut frames = Vec::new();
            let mut endpoints = BTreeMap::new();

            for (n, line) in text.lines().enumerate() {
                let at = format!("{name}:{}", n + 1);
                let line = line.trim_end();
                if line.is_empty() {
                    continue;
                }

                // `# ep2/IN: 1731 urbs, 135698 bytes`
                if let Some(rest) = line.strip_prefix("# ep") {
                    let Some((endpoint, totals)) = rest.split_once(": ") else {
                        continue;
                    };
                    let mut words = totals.split_whitespace();
                    let urbs = words.next().and_then(|w| w.parse().ok());
                    let _ = words.next();
                    let bytes = words
                        .next()
                        .and_then(|w| w.trim_end_matches(',').parse().ok());
                    if let (Some(urbs), Some(bytes)) = (urbs, bytes) {
                        endpoints.insert(format!("ep{endpoint}"), (urbs, bytes));
                    }
                    continue;
                }
                if line.starts_with('#') {
                    continue;
                }

                let (frame, count) = line
                    .split_once('\t')
                    .unwrap_or_else(|| panic!("{at}: expected '<O|I><bytes>\\t<count>'"));
                let direction = match &frame[..1] {
                    "O" => Dir::Out,
                    "I" => Dir::In,
                    other => panic!("{at}: unknown direction {other:?}"),
                };
                let size: usize = frame[1..]
                    .parse()
                    .unwrap_or_else(|e| panic!("{at}: bad payload size: {e}"));
                let count: usize = count
                    .trim()
                    .parse()
                    .unwrap_or_else(|e| panic!("{at}: bad run length: {e}"));
                frames.extend(std::iter::repeat_n((direction, size), count));
            }

            assert!(!frames.is_empty(), "{name} records no frames");
            Shape {
                name,
                frames,
                endpoints,
            }
        }

        fn endpoints(&self) -> Vec<&str> {
            self.endpoints.keys().map(String::as_str).collect()
        }

        /// The capture rode the vendor bulk pair and nothing else, so every line in it is
        /// a protocol frame.
        fn is_vendor_only(&self) -> bool {
            self.endpoints() == ["ep2/IN", "ep3/OUT"]
        }

        fn sizes(&self, want: Dir) -> Vec<usize> {
            self.frames
                .iter()
                .filter(|(d, _)| *d == want)
                .map(|(_, size)| *size)
                .collect()
        }

        /// The run-length lines add up to the per-endpoint totals the header states.
        ///
        /// This checks the reader above against the file's own arithmetic — a misparse
        /// would otherwise weaken every check built on it into something quieter.
        fn check_against_its_own_totals(&self) {
            for (want, suffix) in [(Dir::In, "/IN"), (Dir::Out, "/OUT")] {
                let (urbs, bytes) = self
                    .endpoints
                    .iter()
                    .filter(|(ep, _)| ep.ends_with(suffix))
                    .fold((0, 0), |(u, b), (_, (eu, eb))| (u + eu, b + eb));
                let sizes = self.sizes(want);
                assert_eq!(
                    sizes.len(),
                    urbs,
                    "{}: header counts {urbs} {suffix} URBs, the lines hold {}",
                    self.name,
                    sizes.len(),
                );
                assert_eq!(
                    sizes.iter().sum::<usize>(),
                    bytes,
                    "{}: header counts {bytes} {suffix} bytes",
                    self.name,
                );
            }
        }

        /// Every frame is a size the wire grammar can produce, and no reply is unbidden.
        fn check_grammar(&self) {
            // A request is a header, its arguments and a CRC; a response inserts a `u32`
            // status word ahead of the arguments, so its floor is four higher.
            let min_request = HEADER_LEN + CRC_LEN;
            let min_response = HEADER_LEN + 4 + CRC_LEN;

            for (at, (direction, size)) in self.frames.iter().enumerate() {
                let floor = match direction {
                    Dir::Out => min_request,
                    Dir::In => min_response,
                };
                assert!(
                    *size >= floor,
                    "{}: frame {at} is {size} bytes, below the {floor} a {direction:?} \
                     frame needs",
                    self.name,
                );
            }

            // Every request draws exactly one reply and the fire-and-forget UI messages
            // draw none, so OUTs can only ever outnumber INs. The excess is the number of
            // progress labels and percentages the capture carries.
            let (outs, ins) = (self.sizes(Dir::Out).len(), self.sizes(Dir::In).len());
            assert!(
                outs >= ins,
                "{}: {ins} replies to {outs} requests — the device answered unbidden",
                self.name,
            );
        }

        /// Driving `ops` through this crate emits exactly the OUT frames the capture
        /// holds, in order.
        fn check_transaction(&self, class: ObjectClass, ops: &[Op]) {
            // With more than one OUT endpoint the file cannot say which line is the
            // vendor one, so the comparison would be against a mixture.
            let out_endpoints: Vec<&str> = self
                .endpoints()
                .into_iter()
                .filter(|ep| ep.ends_with("/OUT"))
                .collect();
            assert_eq!(
                out_endpoints,
                ["ep3/OUT"],
                "{}: expected the vendor bulk OUT alone, got {out_endpoints:?}",
                self.name,
            );

            assert_eq!(
                emitted(class, ops),
                self.sizes(Dir::Out),
                "{}: the frames this crate sends do not match the capture",
                self.name,
            );
        }
    }

    // -----------------------------------------------------------------------
    // Replaying a transaction
    // -----------------------------------------------------------------------

    /// One operation NSM performed inside a captured transaction. Addresses are in the
    /// instrument's one-indexed numbering, matching the capture directory names.
    #[derive(Clone, Copy)]
    enum Op {
        Delete(u32, u32),
        Dependencies(u32, u32),
        Duplicate(u32, u32, u32, u32),
        Info(u32, u32),
        Move(u32, u32, u32, u32),
        Rename(u32, u32, &'static str),
        Select(u32, u32),
        /// The counter re-read NSM closes most transactions with. Not part of any
        /// operation — it repaints the host's browser — so [`op`] does not send it and a
        /// caller reproducing NSM asks for it explicitly, as this does.
        Status,
    }

    use Op::{Delete, Dependencies, Duplicate, Info, Move, Rename, Select, Status};

    /// `(capture directory under `usb/`, the class NSM opened, what it ran inside)`.
    ///
    /// The addresses and the rename strings come from each capture's own README, which is
    /// where the operator recorded what they did.
    const TRANSACTIONS: &[(&str, ObjectClass, &[Op])] = &[
        (
            "program/bulk_delete_5-42_43_44",
            ObjectClass::Program,
            &[Delete(5, 42), Delete(5, 43), Delete(5, 44), Status],
        ),
        (
            "program/bulk_duplicate_6-48_49_50_to_7-4_5_6",
            ObjectClass::Program,
            &[
                Duplicate(6, 48, 7, 4),
                Duplicate(6, 49, 7, 5),
                Duplicate(6, 50, 7, 6),
                Info(7, 4),
                Dependencies(7, 4),
                Info(7, 5),
                Dependencies(7, 5),
                Info(7, 6),
                Dependencies(7, 6),
                Status,
            ],
        ),
        (
            "program/bulk_move_7-21_22_23_to_5-42_43_44",
            ObjectClass::Program,
            &[
                Move(7, 21, 5, 42),
                Move(7, 22, 5, 43),
                Move(7, 23, 5, 44),
                Status,
            ],
        ),
        (
            "program/bulk_rename_7-4_5_6",
            ObjectClass::Program,
            &[
                Rename(7, 4, "foo"),
                Rename(7, 5, "bar"),
                Rename(7, 6, "baz"),
                Status,
            ],
        ),
        (
            "program/delete_prog_bank7_loc50",
            ObjectClass::Program,
            &[Delete(7, 50), Status],
        ),
        (
            "program/duplicate_prog_7-2_to_7-3",
            ObjectClass::Program,
            &[
                Duplicate(7, 2, 7, 3),
                Info(7, 3),
                Dependencies(7, 3),
                Status,
            ],
        ),
        (
            "program/move_prog_8-13_to_7-16",
            ObjectClass::Program,
            &[Move(8, 13, 7, 16), Status],
        ),
        // The two select captures close without the counter re-read: nothing stored
        // changed, so NSM has no numbers to repaint.
        (
            "program/open_on_device_2-12",
            ObjectClass::Program,
            &[Select(2, 12)],
        ),
        (
            "program/rename_prog_6-13",
            ObjectClass::Program,
            &[Rename(6, 13, "foo"), Status],
        ),
        (
            "set_list/delete_setlist_4-50",
            ObjectClass::SetList,
            &[Delete(4, 50), Status],
        ),
        (
            "set_list/duplicate_setlist_1-4_to_1-8",
            ObjectClass::SetList,
            &[
                Duplicate(1, 4, 1, 8),
                Info(1, 8),
                Dependencies(1, 8),
                Status,
            ],
        ),
        (
            "set_list/move_setlist_1-3_to_4-50",
            ObjectClass::SetList,
            &[Move(1, 3, 4, 50), Status],
        ),
        (
            "set_list/rename_setlist_1-3",
            ObjectClass::SetList,
            &[Rename(1, 3, "foo"), Status],
        ),
        (
            "set_list/select_setlist_1-2",
            ObjectClass::SetList,
            &[Select(1, 2)],
        ),
    ];

    /// Captures with no `transaction` trial, and the reason. Held to the corpus by
    /// [`check_every_capture_is_accounted_for`], so a new capture cannot land unexamined.
    const NOT_REPRODUCED: &[(&str, &str)] = &[
        (
            "device/firmware_update",
            "no operation — a firmware image over the wire",
        ),
        (
            "device/nsm_startup_sync",
            "NSM's whole browser refresh, hundreds of transactions",
        ),
        (
            "backup/full_backup",
            "many transactions across every object class",
        ),
        (
            "bundle/bundle_download_7-1_4_5",
            "a chunked transfer with NSM's browser reads",
        ),
        (
            "bundle/bundle_upload_7-4_5_6",
            "a chunked transfer with NSM's browser reads",
        ),
        (
            "program/bulk_read_7-1_4_5",
            "a chunked transfer with NSM's browser reads",
        ),
        (
            "program/bulk_write_7-21_22_23",
            "a chunked transfer with NSM's browser reads",
        ),
        (
            "program/read_prog_bank8_loc14",
            "two transactions; the bytes are replayed in status.rs",
        ),
        (
            "program/write_prog_bank7_loc50",
            "a chunked transfer plus a bank-refresh transaction",
        ),
        ("program/bulk_relink_piano_7-4_5_6", RELINK),
        ("program/bulk_relink_sample_7-4_5_6", RELINK),
        ("program/relink_piano_clavinet_7-4", RELINK),
        ("program/relink_piano_harps_7-4", RELINK),
        ("program/relink_piano_prog7-4", RELINK),
        ("program/relink_sample_8-4", RELINK),
        ("program/relink_sample_prog7-50", RELINK),
        ("set_list/relink_setlist_1-4_A_to_6-13", RELINK),
        ("set_list/relink_setlist_1-4_B_to_1-5", RELINK),
        ("set_list/relink_setlist_1-4_C_to_2-26", RELINK),
        ("set_list/relink_setlist_1-4_D_to_1-37", RELINK),
    ];

    const RELINK: &str = "command 0x35's payload is not pinned down, so no typed operation \
                          exists to drive";

    /// Every capture that ships a shape file is either reproduced or listed as not.
    fn check_every_capture_is_accounted_for(root: &Path) {
        let captured: BTreeSet<String> = shape_files(root)
            .iter()
            .map(|p| {
                capture_name(root, p)
                    .strip_prefix("usb/")
                    .expect("captures live under usb/")
                    .to_string()
            })
            .collect();
        let accounted: BTreeSet<String> = TRANSACTIONS
            .iter()
            .map(|(c, ..)| c.to_string())
            .chain(NOT_REPRODUCED.iter().map(|(c, _)| c.to_string()))
            .collect();

        let unexamined: Vec<_> = captured.difference(&accounted).collect();
        assert!(
            unexamined.is_empty(),
            "captures with no verdict — reproduce them or list them in NOT_REPRODUCED: \
             {unexamined:?}",
        );

        let gone: Vec<_> = accounted.difference(&captured).collect();
        assert!(
            gone.is_empty(),
            "listed captures the corpus no longer holds: {gone:?}",
        );
    }

    /// The payload size of every frame the host sent, running `ops` inside one
    /// transaction on `class`.
    fn emitted(class: ObjectClass, ops: &[Op]) -> Vec<usize> {
        let mut device = Responder::default();
        block_on(async {
            let mut session = Session::open(&mut device, class)
                .await
                .expect("the responder answered the open")
                .allow_destructive_writes();

            for step in ops {
                let done = match *step {
                    Delete(b, s) => op::delete(&mut session, Location::from_user(b, s)).await,
                    Dependencies(b, s) => op::dependencies(&mut session, Location::from_user(b, s))
                        .await
                        .map(|_| ()),
                    Duplicate(fb, fs, tb, ts) => {
                        op::duplicate(
                            &mut session,
                            Location::from_user(fb, fs),
                            Location::from_user(tb, ts),
                        )
                        .await
                    }
                    Info(b, s) => op::info(&mut session, Location::from_user(b, s))
                        .await
                        .map(|_| ()),
                    Move(fb, fs, tb, ts) => {
                        op::move_object(
                            &mut session,
                            Location::from_user(fb, fs),
                            Location::from_user(tb, ts),
                        )
                        .await
                    }
                    Rename(b, s, name) => {
                        op::rename(&mut session, Location::from_user(b, s), name).await
                    }
                    Select(b, s) => op::select(&mut session, Location::from_user(b, s)).await,
                    Status => op::status(&mut session).await.map(|_| ()),
                };
                done.expect("the responder answered the operation");
            }

            session
                .commit()
                .await
                .expect("the responder answered the close");
        });
        device.sent
    }

    /// A device that answers every request with a well-formed, content-free reply.
    ///
    /// ⚠️ Asserts nothing about a real instrument, and is not a model of one. Its whole
    /// job is to let the code under test run to completion so what the **host** sent can
    /// be measured — the shape files record sizes, not bytes, so there is no captured
    /// response to replay. What it does enforce along the way is that every frame the
    /// host emits decodes: its declared length and its CRC have to agree with its bytes.
    #[derive(Default)]
    struct Responder {
        sent: Vec<usize>,
        /// The request still awaiting a reply. A fire-and-forget message overwrites it
        /// and is never read back, which is exactly how the device treats one.
        pending: Option<Message>,
    }

    impl Transport for Responder {
        async fn write(&mut self, buf: &[u8]) -> Result<()> {
            self.sent.push(buf.len());
            self.pending = Some(Message::decode(buf)?);
            Ok(())
        }

        async fn read(&mut self, _max: usize) -> Result<Vec<u8>> {
            let request = self
                .pending
                .take()
                .ok_or_else(|| Error::Transport("the host read without asking".into()))?;
            Ok(reply(&request))
        }
    }

    /// The smallest reply each request will accept: `command + 1`, a success status, and
    /// whatever shape the decoder for that command insists on.
    fn reply(request: &Message) -> Vec<u8> {
        let mut args = vec![0u8; 4]; // status word, success
        match (request.service, request.command) {
            // `count, free, used`, and the two further words the device sends.
            (Service::Program, cmd::STATUS) => args.extend_from_slice(&[0u8; 20]),
            // bank, slot, body_len, format tag, version, two opaque words, name length.
            (Service::Program, cmd::INFO) => {
                args.extend_from_slice(&[0u8; 12]);
                args.extend_from_slice(b"ne5t");
                args.extend_from_slice(&[0u8; 16]);
            }
            // bank, slot, and an empty dependency list.
            (Service::Program, cmd::DEPENDENCIES) => args.extend_from_slice(&[0u8; 12]),
            // Everything else echoes its arguments back, as the device does.
            _ => args.extend_from_slice(request.payload()),
        }
        Message::new(
            request.service,
            request.subsystem,
            request.command + 1,
            args,
        )
        .encode()
    }

    // -----------------------------------------------------------------------
    // Set-list bodies
    // -----------------------------------------------------------------------

    /// An 18-byte `.ne5t` body read straight off the instrument, taken through both
    /// crates and back.
    ///
    /// The wire carries the body alone — the CBIN header is the host's to build — so this
    /// is the seam: [`envelope::wrap`] must produce a header `nord-format` accepts, and
    /// `nord-format`'s writer must reproduce the body the device sent, bit for bit.
    ///
    /// The version is not in the body's own right: it lives in the header at `0x14` and
    /// is *echoed* into bit 48 of the map word. Wrapping with the other version and
    /// watching the body stop reproducing is what shows the echo is load-bearing rather
    /// than decorative.
    fn check_set_list_body(path: &Path) {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let body = std::fs::read(path).unwrap();
        assert_eq!(body.len(), 18, "{name}: not an 18-byte set-list body");

        // `<bank>-<slot>_<Name>.ne5t.body`, as the instrument displays the address.
        let (bank, slot) = name
            .split_once('_')
            .and_then(|(at, _)| at.split_once('-'))
            .map(|(b, s)| (b.parse::<u32>().unwrap(), s.parse::<u32>().unwrap()))
            .unwrap_or_else(|| panic!("{name}: expected a <bank>-<slot>_<name> filename"));
        let at = Location::from_user(bank, slot);

        // The version the map word echoes, which is the one the header must carry.
        let version = u32::from(u16::from_be_bytes([body[0], body[1]]));
        assert!(version <= 1, "{name}: unexpected version echo {version}");

        let file = envelope::wrap("ne5t", at, version, &body).unwrap();
        let entity = nord_format::from_stream(&mut std::io::Cursor::new(&file))
            .unwrap_or_else(|e| panic!("{name}: the wrapped body did not parse: {e}"));

        let nord_format::Entity::Song(nord_format::Song::Electro5(song)) = &entity else {
            panic!("{name}: wrapped as a set list, decoded as {entity:?}")
        };
        {
            use nord_format::common::bank::Item;
            assert_eq!(
                song.location().inner(),
                ((bank - 1) as u16, (slot - 1) as u16),
                "{name}: the slot the envelope stamped did not survive the decode",
            );
        }
        assert_eq!(song.version(), version, "{name}: version");

        let emitted = nord_format::to_bytes(&entity).unwrap();
        let (_, _, rewritten) = envelope::unwrap(&emitted).unwrap();
        assert_eq!(
            rewritten, body,
            "{name}: the re-emitted body is not what the device sent",
        );

        // Same body, wrong version: the echo must move, or bit 48 is not carrying it.
        let other = envelope::wrap("ne5t", at, 1 - version, &body).unwrap();
        let entity = nord_format::from_stream(&mut std::io::Cursor::new(&other)).unwrap();
        let emitted = nord_format::to_bytes(&entity).unwrap();
        let (_, _, echoed) = envelope::unwrap(&emitted).unwrap();
        assert_ne!(
            echoed, body,
            "{name}: the header version does not reach the body, so the echo at bit 48 \
             is not being written",
        );
    }
}

fn main() {
    #[cfg(feature = "corpus")]
    shapes::run();
}
