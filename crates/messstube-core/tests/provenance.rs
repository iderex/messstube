//! The provenance block seen from outside the crate, from #36.
//!
//! These are here rather than beside the code because what they assert is what
//! a caller of the library gets, and a caller is outside the crate. The two
//! properties that matter to somebody keeping a converted file for ten years
//! are that the block is always there and that it does not change between runs,
//! and neither is visible from inside a module.
//!
//! The fixture reader below is written the way a reader crate would be written,
//! against the published interface and nothing else, so that a mechanism which
//! only holds inside the crate would fail here.

// An integration test is its own crate and does not inherit the crate root's
// attribute, so it says so itself.
#![forbid(unsafe_code)]
// Turned off for this test target only, the same way the other test targets do
// it: a test whose own premise did not hold has to stop loudly rather than go
// on and report a difference it cannot have measured.
#![allow(clippy::panic)]

use messstube_core::error::{ReadError, ReadOptions, ReadOutcome};
use messstube_core::measurement::Measurement;
use messstube_core::provenance::Instrument;
use messstube_core::read::read_with;
use messstube_core::reader::{Family, Maturity, Reader, Source};

/// Sixteen bytes, written as an escaped literal under the rule in
/// `docs/testing.md`: the pair at offsets 4 and 5 is what a checkout would
/// rewrite if this were a file in the tree, and the digest would then be of
/// different bytes on different platforms while every assertion below still
/// passed.
const INPUT: &[u8] = b"MSTB\x0d\x0a\x01\x00\x10\x20\x30\x40\x50\x60\x70\x80";

/// A reader that accepts the fixture and reports what the file said about the
/// instrument, which is the only part of the block a reader supplies.
struct Fixture {
    instrument: Option<Instrument>,
}

impl Reader for Fixture {
    fn id(&self) -> String {
        "fixture".to_owned()
    }
    fn name(&self) -> String {
        "the provenance fixture".to_owned()
    }
    fn family(&self) -> Family {
        Family::Oscilloscope
    }
    fn maturity(&self) -> Maturity {
        Maturity::Sketched
    }
    fn recognises(&self, prefix: &[u8]) -> bool {
        prefix.starts_with(b"MSTB")
    }
    fn read(
        &self,
        _source: &mut dyn Source,
        _options: ReadOptions,
    ) -> Result<ReadOutcome, ReadError> {
        let mut measurement = Measurement::new(Vec::new(), Vec::new());
        measurement.instrument.clone_from(&self.instrument);
        Ok(ReadOutcome::complete(measurement))
    }
}

/// The block as the bytes it would be written to a file as, which is what
/// "byte-identical" is a claim about.
fn rendered(outcome: &ReadOutcome) -> Vec<u8> {
    let Some(provenance) = outcome.measurement.provenance() else {
        panic!("the read path attaches a block to every measurement it produces")
    };
    let mut written = String::new();
    for (name, value) in provenance.fields() {
        written.push_str(&name);
        written.push_str(": ");
        written.push_str(&value);
        written.push('\n');
    }
    written.into_bytes()
}

fn read_once(instrument: Option<Instrument>) -> ReadOutcome {
    let reader = Fixture { instrument };
    let mut source = std::io::Cursor::new(INPUT.to_vec());
    match read_with(&reader, "run-14.mstb", &mut source, ReadOptions::default()) {
        Ok(outcome) => outcome,
        Err(failure) => panic!("the fixture reader accepts this input: {failure}"),
    }
}

#[test]
fn two_reads_of_one_input_produce_the_same_provenance() {
    // The property the omissions exist for. A conversion timestamp, a hostname
    // or an account name in the block would red this line, which is why the
    // omission is a test rather than a paragraph.
    let first = rendered(&read_once(None));
    let second = rendered(&read_once(None));
    assert_eq!(first, second, "two runs over one input disagreed");

    // And it is not vacuously equal because the block is empty.
    let text = String::from_utf8_lossy(&first).into_owned();
    assert!(text.contains("input: run-14.mstb"), "{text}");
    assert!(text.contains("input length in bytes: 16"), "{text}");
    assert!(text.contains("content hash algorithm: SHA-256"), "{text}");
    assert!(text.contains("reader: fixture"), "{text}");
    assert!(text.contains("reader maturity: sketched"), "{text}");
}

#[test]
fn every_measurement_off_the_read_path_carries_a_block() {
    // Whatever the reader did or did not do. The reader above sets no
    // provenance and cannot, and the measurement still comes back with one.
    let outcome = read_once(None);
    assert!(outcome.measurement.provenance().is_some());

    // A measurement built by a caller, or handed back by a reader called
    // directly, carries none, and that is the honest answer rather than an
    // empty block: nothing hashed the input, so there is nothing to record.
    let bare = Measurement::new(Vec::new(), Vec::new());
    assert!(bare.provenance().is_none());
}

#[test]
fn the_digest_in_the_block_is_of_the_input_the_caller_named() {
    // The expected value was obtained outside this repository, from the
    // platform's own checksum tool over the same sixteen bytes:
    //
    //     printf 'MSTB\r\n\x01\x00\x10\x20\x30\x40\x50\x60\x70\x80' | sha256sum
    //     a8a019306de3d60b7d980953ad35365a7d5fdd87d8ac2e847747a5ce2d50e4bb *-
    //
    // That is what makes this an independently obtained value rather than a
    // record of what this code happened to print. The digest a person checks a
    // converted file with is produced by a tool like that one, so it is the
    // right thing to agree with.
    const EXPECTED: &str = "a8a019306de3d60b7d980953ad35365a7d5fdd87d8ac2e847747a5ce2d50e4bb";

    let outcome = read_once(None);
    let Some(provenance) = outcome.measurement.provenance() else {
        panic!("the read path attaches a block to every measurement it produces")
    };
    assert_eq!(provenance.length, 16);
    assert_eq!(provenance.hash.digest, EXPECTED);
    assert_eq!(provenance.hash.to_string(), format!("SHA-256:{EXPECTED}"));
}

#[test]
fn the_instrument_a_reader_found_is_the_only_part_it_contributes() {
    let found = Instrument {
        manufacturer: Some("Tektronix".to_owned()),
        serial: Some("C010203".to_owned()),
        ..Instrument::default()
    };
    let outcome = read_once(Some(found.clone()));
    let Some(provenance) = outcome.measurement.provenance() else {
        panic!("the read path attaches a block to every measurement it produces")
    };

    assert_eq!(provenance.instrument, Some(found));
    // The rest of the block came from the read path, and a reader declaring a
    // higher maturity level in the block than it declares in code is not a
    // thing that can be written.
    assert_eq!(provenance.maturity, Maturity::Sketched);
    assert_eq!(provenance.reader, "fixture");
}
