//! The two error kinds, proved against fixtures, from #34.
//!
//! What is asserted here is not that the type has two variants, which the
//! compiler already says. It is that a reader handed different bytes arrives at
//! different kinds, that the kinds are decided by whether the file was
//! recognised rather than by how badly it was broken, and that a caller who did
//! not ask for a partial read cannot receive one.
//!
//! THE READER BELOW IS A FIXTURE READER AND NOTHING ELSE. Its format is
//! invented here, in this file, and no instrument writes it. That is deliberate:
//! a test that judged against a real reader would prove the state of that reader
//! on the day it ran, and what has to be proved is the error model, which is
//! fixed before the first reader exists. `docs/testing.md` fixes the other half,
//! that the bytes go in as escaped literals rather than as files in the tree, so
//! that a checkout cannot rewrite one.
//!
//! The fixture format, so the offsets below can be read against it:
//!
//! ```text
//! 0   4 bytes   the magic, "MSTB"
//! 4   1 byte    how many channels follow
//! then, per channel:
//!     1 byte    the length of the name
//!     n bytes   the name
//!     2 bytes   how many samples follow, big endian
//!     2m bytes  the samples, signed 16-bit big endian
//! ```

// The library's own crate root forbids unsafe code. A test target is a separate
// crate and does not inherit that, so it says so itself.
#![forbid(unsafe_code)]
// Turned off for this test target only, the same way the library's unit test
// modules do it: a test whose precondition does not hold has to stop loudly, and
// `expect` with a sentence in it says which precondition that was.
#![allow(clippy::expect_used)]
// And the same for `panic`, which is how the match arms below say that a fixture
// carrying the reader's own magic came back as an unrecognised file. That is a
// failure of the test's own premise rather than an assertion, and it has to stop
// the run rather than be reported as a difference.
#![allow(clippy::panic)]

use messstube_core::error::{Loss, ReadError, ReadOptions, ReadOutcome};
use messstube_core::measurement::{Axis, AxisShape, Channel, Measurement, Samples, Transform};
use messstube_core::unit::Unit;

/// The identifier the fixture reader declares. A real one is declared through
/// the reader interface in #32; this is the string such a reader would put in
/// the errors below.
const READER: &str = "fixture-toy";

/// The four bytes that decide recognition.
const MAGIC: [u8; 4] = [0x4d, 0x53, 0x54, 0x42];

/// A whole file: two channels of two samples each, twenty-five bytes.
///
/// `Ch1` holds 10 and 20, `Ch2` holds 30 and 40. The second channel's record
/// begins at offset 15 and its samples at offset 21.
const WHOLE: &[u8] = b"MSTB\x02\x03Ch1\x00\x02\x00\x0a\x00\x14\x03Ch2\x00\x02\x00\x1e\x00\x28";

/// The same file with the last two bytes cut off, twenty-three bytes.
///
/// `Ch2` promises two samples, one of them is there, and the file ends at offset
/// 23 in the middle of the second.
const TRUNCATED: &[u8] = b"MSTB\x02\x03Ch1\x00\x02\x00\x0a\x00\x14\x03Ch2\x00\x02\x00\x1e";

/// [`TRUNCATED`] with one byte of the magic changed, and nothing else.
///
/// This is the near miss the error model exists to get right. It is broken in
/// exactly the same place and in exactly the same way, and it is a file this
/// reader has no business having an opinion about. A model that decided the kind
/// from the damage would call it damaged.
const OTHER_FORMAT: &[u8] = b"MSTC\x02\x03Ch1\x00\x02\x00\x0a\x00\x14\x03Ch2\x00\x02\x00\x1e";

/// Three bytes, which is one short of the magic itself.
///
/// A file too short to recognise is not recognised. It is not damaged, because
/// nothing has established that this reader was ever the right one.
const SHORTER_THAN_THE_MAGIC: &[u8] = b"MST";

/// The refusal this reader gives to a file it does not recognise.
fn declined() -> ReadError {
    ReadError::NotThisFormat {
        reader: READER.to_owned(),
    }
}

/// An offset as the error model carries it: counted from the start of the input
/// and never from the start of whatever record the reader was inside.
fn absolute(offset: usize) -> u64 {
    // `try_from` rather than a cast, because the workspace lint set denies a
    // lossy one and a fixture is not a reason to make an exception. No fixture
    // in this file is near the bound.
    u64::try_from(offset).unwrap_or(u64::MAX)
}

/// What the reader says when the file ended before it had what it needed.
fn ended_early(at: usize, expected: &str, total: usize) -> ReadError {
    ReadError::Damaged {
        reader: READER.to_owned(),
        offset: absolute(at),
        expected: expected.to_owned(),
        found: format!("the end of a file of {total} bytes"),
    }
}

/// The fixture reader.
///
/// It recognises by the magic, refuses with an absolute offset when the file
/// stops early, and returns what it recovered plus a loss list when the caller
/// asked for a partial read. It synthesises nothing: a channel whose samples ran
/// out is not returned short, it is left out and named in the losses.
#[allow(clippy::too_many_lines)]
fn read_fixture(bytes: &[u8], options: ReadOptions) -> Result<ReadOutcome, ReadError> {
    let total = bytes.len();
    let mut at = 0_usize;

    // Recognition first, and it is the only thing that can produce the declining
    // kind. Everything after this point is about a file this reader has claimed.
    if bytes.get(at..at + MAGIC.len()) != Some(MAGIC.as_slice()) {
        return Err(declined());
    }
    at += MAGIC.len();

    let channel_count = *bytes
        .get(at)
        .ok_or_else(|| ended_early(at, "the channel count", total))?;
    at += 1;

    let mut channels: Vec<Channel> = Vec::new();
    let mut losses: Vec<Loss> = Vec::new();

    for _ in 0..usize::from(channel_count) {
        let Some(&length) = bytes.get(at) else {
            let reason = "the length of a channel name";
            if options.partial_reads_requested() {
                losses.push(Loss {
                    offset: absolute(at),
                    reason: reason.to_owned(),
                    channel: None,
                    ended_at_sample: None,
                });
                break;
            }
            return Err(ended_early(at, reason, total));
        };
        let name_length = usize::from(length);
        at += 1;

        let Some(name) = bytes
            .get(at..at + name_length)
            .map(|raw| String::from_utf8_lossy(raw).into_owned())
        else {
            let reason = "a channel name";
            if options.partial_reads_requested() {
                losses.push(Loss {
                    offset: absolute(at),
                    reason: reason.to_owned(),
                    channel: None,
                    ended_at_sample: None,
                });
                break;
            }
            return Err(ended_early(at, reason, total));
        };
        at += name_length;

        let Some(declared) = bytes
            .get(at..at + 2)
            .and_then(|raw| <[u8; 2]>::try_from(raw).ok())
        else {
            let reason = "the sample count of a channel";
            if options.partial_reads_requested() {
                losses.push(Loss {
                    offset: absolute(at),
                    reason: reason.to_owned(),
                    channel: Some(name),
                    ended_at_sample: None,
                });
                break;
            }
            return Err(ended_early(at, reason, total));
        };
        let sample_count = usize::from(u16::from_be_bytes(declared));
        at += 2;

        // Not `with_capacity(sample_count)`. That count came out of the file,
        // and part one of `docs/decisions/0007-hostile-input-budget.md` refuses
        // an allocation sized from a number the input supplied. The checked
        // helper that makes such a size legal is #35; a fixture reader is not a
        // reason to write the shape the budget exists against.
        let mut samples: Vec<i16> = Vec::new();
        let mut stopped_at: Option<usize> = None;
        for index in 0..sample_count {
            let Some(raw) = bytes
                .get(at..at + 2)
                .and_then(|raw| <[u8; 2]>::try_from(raw).ok())
            else {
                stopped_at = Some(index);
                break;
            };
            samples.push(i16::from_be_bytes(raw));
            at += 2;
        }

        if let Some(index) = stopped_at {
            let reason = format!("sample {index} of the {sample_count} this channel promised");
            if options.partial_reads_requested() {
                // The channel is NOT pushed. What was read of it is dropped
                // rather than returned short, which is the whole of "nothing is
                // synthesised": eleven samples where two thousand were promised
                // is a measurement of a different thing, and a caller cannot
                // tell it from a short recording.
                losses.push(Loss {
                    offset: absolute(at),
                    reason,
                    channel: Some(name),
                    ended_at_sample: Some(index),
                });
                break;
            }
            return Err(ended_early(at, &reason, total));
        }

        channels.push(Channel {
            name,
            unit: Unit::Volt,
            samples: Samples::I16(samples),
            transform: Transform::IDENTITY,
            uncertainty: None,
        });
    }

    let length = channels.first().map_or(0, |channel| channel.samples.len());
    let measurement = Measurement::new(
        channels,
        vec![Axis {
            name: "sample".to_owned(),
            unit: Unit::Dimensionless,
            shape: AxisShape::Regular {
                start: 0.0,
                step: 1.0,
                count: length,
            },
        }],
    );

    Ok(ReadOutcome {
        measurement,
        losses,
    })
}

#[test]
fn a_whole_file_comes_back_whole_and_with_nothing_lost() {
    let outcome = read_fixture(WHOLE, ReadOptions::default()).expect("the whole fixture is whole");

    assert!(outcome.is_complete());
    assert!(outcome.losses.is_empty());
    assert_eq!(outcome.measurement.channels.len(), 2);

    let second = outcome
        .measurement
        .channels
        .get(1)
        .expect("the whole fixture holds two channels");
    assert_eq!(second.name, "Ch2");
    assert_eq!(second.samples, Samples::I16(vec![30, 40]));
}

#[test]
fn a_file_this_reader_does_not_recognise_is_declined_and_never_called_damaged() {
    // The two kinds point a user in opposite directions. This one says try
    // something else.
    let error = read_fixture(OTHER_FORMAT, ReadOptions::default())
        .expect_err("the magic of this fixture is not the one the reader claims");

    assert_eq!(
        error,
        ReadError::NotThisFormat {
            reader: READER.to_owned()
        }
    );
}

#[test]
fn a_file_shorter_than_the_magic_is_declined_rather_than_damaged() {
    // Nothing has established that this reader was ever the right one, so it has
    // no standing to call the file broken.
    let error = read_fixture(SHORTER_THAN_THE_MAGIC, ReadOptions::default())
        .expect_err("three bytes cannot carry a four byte magic");

    assert_eq!(
        error,
        ReadError::NotThisFormat {
            reader: READER.to_owned()
        }
    );
}

#[test]
fn a_recognised_file_that_stops_early_is_damaged_and_says_what_it_wanted() {
    let error = read_fixture(TRUNCATED, ReadOptions::default())
        .expect_err("the truncated fixture ends inside its samples");

    match error {
        ReadError::Damaged {
            reader,
            offset,
            expected,
            found,
        } => {
            assert_eq!(reader, READER);
            assert_eq!(offset, 23);
            // The expectation is in the file's own vocabulary rather than the
            // reader's: which sample of how many, not which function returned.
            assert_eq!(expected, "sample 1 of the 2 this channel promised");
            assert_eq!(found, "the end of a file of 23 bytes");
        }
        ReadError::NotThisFormat { reader } => {
            panic!("{reader} declined a file carrying its own magic");
        }
    }
}

#[test]
fn an_offset_is_absolute_in_the_file() {
    // The fixture is built so the two candidate answers differ. `Ch2`'s record
    // begins at offset 15 and its samples at offset 21, so a reader reporting
    // where it stopped relative to the record would say 8, and relative to the
    // sample block would say 2. The absolute answer is 23, which is also the
    // length of the file and therefore the offset a hex editor puts the cursor
    // at.
    assert_eq!(TRUNCATED.len(), 23);

    let error = read_fixture(TRUNCATED, ReadOptions::default())
        .expect_err("the truncated fixture ends inside its samples");

    let ReadError::Damaged { offset, .. } = error else {
        panic!("the truncated fixture carries the reader's own magic");
    };
    assert_eq!(offset, 23, "an offset of 8 or 2 would be a relative one");

    // And the sentence a caller reads carries that same number, because the
    // number in the message is what somebody types into the hex editor.
    let sentence = read_fixture(TRUNCATED, ReadOptions::default())
        .expect_err("the truncated fixture ends inside its samples")
        .to_string();
    assert!(sentence.contains("byte 23"), "{sentence}");
}

#[test]
fn the_two_kinds_are_decided_by_recognition_and_not_by_the_damage() {
    // The pair the issue asks for. These two fixtures are broken identically and
    // differ in one byte, and that byte is the one recognition turns on. A model
    // that decided the kind from how the file ended would give the same answer
    // for both.
    assert_eq!(TRUNCATED.len(), OTHER_FORMAT.len());
    let differing = TRUNCATED
        .iter()
        .zip(OTHER_FORMAT.iter())
        .filter(|(left, right)| left != right)
        .count();
    assert_eq!(
        differing, 1,
        "the two fixtures differ in more than the magic"
    );

    let damaged = read_fixture(TRUNCATED, ReadOptions::default())
        .expect_err("the truncated fixture carries the magic");
    let declined = read_fixture(OTHER_FORMAT, ReadOptions::default())
        .expect_err("the other fixture does not carry the magic");

    assert!(matches!(damaged, ReadError::Damaged { .. }));
    assert!(matches!(declined, ReadError::NotThisFormat { .. }));
    assert_ne!(damaged, declined);
}

#[test]
fn a_partial_read_returns_what_was_whole_and_names_what_was_not() {
    let outcome = read_fixture(TRUNCATED, ReadOptions::default().partial_reads(true))
        .expect("a partial read of a recognised file returns what it recovered");

    assert!(!outcome.is_complete());

    // What was recovered is recovered in full, and what was not is absent
    // rather than short. `Ch2` had one of its two samples in the file, and the
    // one thing that may not happen is a channel called `Ch2` holding one
    // sample: a caller cannot tell that from a recording of one sample.
    assert_eq!(outcome.measurement.channels.len(), 1);
    let recovered = outcome
        .measurement
        .channels
        .first()
        .expect("one channel was recovered");
    assert_eq!(recovered.name, "Ch1");
    assert_eq!(recovered.samples, Samples::I16(vec![10, 20]));
    assert!(
        !outcome
            .measurement
            .channels
            .iter()
            .any(|channel| channel.name == "Ch2"),
        "the half-read channel came back truncated instead of being reported lost"
    );

    // And the loss says where and how far it got, which is what a truncated
    // vector cannot say.
    assert_eq!(
        outcome.losses,
        vec![Loss {
            offset: 23,
            reason: "sample 1 of the 2 this channel promised".to_owned(),
            channel: Some("Ch2".to_owned()),
            ended_at_sample: Some(1),
        }]
    );
}

#[test]
fn a_caller_who_did_not_ask_for_a_partial_read_cannot_receive_one() {
    // The same bytes, the same reader, the default options. A caller who did not
    // ask will not check whether they got one, so they get a refusal instead.
    let refused = read_fixture(TRUNCATED, ReadOptions::default());
    assert!(refused.is_err());

    let asked = read_fixture(TRUNCATED, ReadOptions::default().partial_reads(true));
    assert!(asked.is_ok());

    // And the two agree about where the file stopped, so the offset a partial
    // caller reads is the offset a refusing caller reads.
    let ReadError::Damaged { offset, .. } = refused.expect_err("the default refuses") else {
        panic!("the truncated fixture carries the reader's own magic");
    };
    let losses = asked.expect("the asking caller got an outcome").losses;
    let loss = losses.first().expect("the partial read reported one loss");
    assert_eq!(offset, loss.offset);
}
