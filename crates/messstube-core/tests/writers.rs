//! The two writers seen from outside the crate, from #38.
//!
//! What is asserted here is the bytes. A writer whose output is described rather
//! than pinned is a writer whose output changes without anybody noticing, and
//! the whole argument for plain text in
//! `docs/decisions/0008-output-and-interchange.md` is that a change in a reader
//! shows up as a diff.

// An integration test is its own crate and does not inherit the crate root's
// attribute, so it says so itself.
#![forbid(unsafe_code)]
// Turned off for this test target only, the same way the other test targets do
// it: a test whose own premise did not hold has to stop loudly rather than go on
// and report a difference it cannot have measured.
#![allow(clippy::expect_used)]
// And the same for `panic`, which is how the round-trip battery below says which
// value failed to read back. That is the test's own premise coming apart rather
// than an assertion, and it has to stop the run.
#![allow(clippy::panic)]

use messstube_core::measurement::{
    Axis, AxisShape, Channel, Measurement, Samples, Transform, Uncertainty,
};
use messstube_core::unit::Unit;
use messstube_core::write::{Values, metadata_document, number, sample_table};

/// Two channels on one regular time axis, which is the oscilloscope shape and
/// the one the first reader will produce.
fn two_channels() -> Measurement {
    Measurement::new(
        vec![
            Channel {
                name: "Ch1".to_owned(),
                unit: Unit::Volt,
                samples: Samples::I16(vec![0, 16_384, -32_768]),
                transform: Transform {
                    scale: 1.0 / 32_768.0,
                    offset: 0.0,
                },
                uncertainty: Some(Uncertainty::Relative(0.01)),
            },
            Channel {
                name: "Ch2".to_owned(),
                unit: Unit::Volt,
                samples: Samples::I16(vec![1, 2, 3]),
                transform: Transform {
                    scale: 1.0,
                    offset: 0.5,
                },
                uncertainty: None,
            },
        ],
        vec![Axis {
            name: "time".to_owned(),
            unit: Unit::Second,
            shape: AxisShape::Regular {
                start: 0.0,
                step: 400e-9,
                count: 3,
            },
        }],
    )
}

#[test]
fn the_table_is_the_bytes_this_test_writes_out() {
    // Pinned rather than described. The header names every column with its
    // unit, the rows are in index order, the separator is a tab and the line
    // ending is a line feed on every platform.
    let table = sample_table(&two_channels(), Values::Physical).expect("these names are writable");
    assert_eq!(
        table,
        "time (s)\tCh1 (V)\tCh2 (V)\n\
         0\t0\t1.5\n\
         4e-7\t0.5\t2.5\n\
         8e-7\t-1\t3.5\n"
    );
}

#[test]
fn the_stored_codes_are_reachable_and_say_they_are_codes() {
    // 0004 keeps both and both have to be reachable: the archive wants the
    // codes, the analysis wants the values. The header is what stops a column
    // of codes being read as volts.
    let table = sample_table(&two_channels(), Values::Stored).expect("these names are writable");
    assert_eq!(
        table,
        "time (s)\tCh1 (stored code)\tCh2 (stored code)\n\
         0\t0\t1\n\
         4e-7\t16384\t2\n\
         8e-7\t-32768\t3\n"
    );
}

#[test]
fn the_same_measurement_writes_the_same_bytes_every_time() {
    // What makes a corpus test a diff. Nothing here reads a clock, a locale or
    // anything else that could differ between two runs, and this is the
    // assertion that says so rather than the paragraph.
    let measurement = two_channels();
    for _ in 0..4 {
        assert_eq!(
            sample_table(&measurement, Values::Physical).expect("these names are writable"),
            sample_table(&two_channels(), Values::Physical).expect("these names are writable")
        );
        assert_eq!(
            metadata_document(&measurement).expect("these names are writable"),
            metadata_document(&two_channels()).expect("these names are writable")
        );
    }
}

#[test]
fn a_written_value_reads_back_as_the_value_that_was_written() {
    // A converter that loses the last bits cannot be used for archiving. The
    // battery is the values that break a naive formatter: the ones with no
    // exact binary representation, the ends of the range, the subnormals, and
    // the two zeroes, which are different values and have to stay different.
    let battery = [
        0.0_f64,
        -0.0,
        1.0,
        -1.0,
        0.1,
        0.2,
        0.1 + 0.2,
        1.0 / 3.0,
        1.0 / 32_768.0,
        400e-9,
        1.234_567_890_123_456_7e-5,
        9.007_199_254_740_993e15,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        5e-324,
        1e-300,
        1e300,
        -2.225_073_858_507_2e-308,
    ];

    for value in battery {
        let written = number(value);
        let read: f64 = written
            .parse()
            .unwrap_or_else(|_| panic!("{written} did not read back as a number"));
        assert_eq!(
            read.to_bits(),
            value.to_bits(),
            "{value} was written as {written} and read back as {read}"
        );
    }

    // The three that are not numbers read back as themselves, and a reader is
    // not permitted to invent one, so the only way they arrive is a file that
    // stated them.
    assert_eq!(number(f64::INFINITY), "inf");
    assert_eq!(number(f64::NEG_INFINITY), "-inf");
    assert_eq!(number(f64::NAN), "NaN");
    assert!(
        number(f64::NAN)
            .parse::<f64>()
            .expect("NaN reads back")
            .is_nan()
    );
}

#[test]
fn the_decimal_separator_is_a_full_stop_and_there_is_no_grouping() {
    // The failure this refuses is silent: a file written with a comma decimal
    // separator is read by the next tool as two columns or as a different
    // number, and it looks like the data rather than like a defect. These are
    // the values that differ between the two conventions.
    assert_eq!(number(1234.5), "1234.5");
    assert_eq!(number(1_000_000.0), "1000000");
    assert_eq!(number(0.001), "0.001");
    assert_eq!(number(-9_876_543.25), "-9876543.25");

    // And nothing anywhere in a written table carries a comma, which is the
    // check that catches a grouping separator arriving through a value rather
    // than through the formatter.
    let table = sample_table(&two_channels(), Values::Physical).expect("these names are writable");
    assert!(!table.contains(','), "{table}");

    // This holds on any machine rather than on the one the gate runs on: the
    // standard library's number formatting takes no locale, so there is no input
    // by which a machine could change these bytes. What could change them is
    // somebody introducing a formatter that does take one, and that is what the
    // assertion above is placed here to red.
}

#[test]
fn a_channel_shorter_than_its_neighbour_writes_an_empty_field_and_not_a_zero() {
    // The rule 0006 states for losses, applied to the table: a zero is
    // indistinguishable from a measurement of zero, and this field is full of
    // measurements that are legitimately zero.
    let measurement = Measurement::new(
        vec![
            Channel {
                name: "long".to_owned(),
                unit: Unit::Volt,
                samples: Samples::I16(vec![1, 2]),
                transform: Transform::IDENTITY,
                uncertainty: None,
            },
            Channel {
                name: "short".to_owned(),
                unit: Unit::Volt,
                samples: Samples::I16(vec![7]),
                transform: Transform::IDENTITY,
                uncertainty: None,
            },
        ],
        Vec::new(),
    );

    let table = sample_table(&measurement, Values::Physical).expect("these names are writable");
    assert_eq!(table, "long (V)\tshort (V)\n1\t7\n2\t\n");
}

#[test]
fn a_name_that_would_move_a_column_is_refused_and_says_which_one() {
    // Repairing the name would put a name in the output the instrument never
    // wrote, and writing it as it stands moves every field after it by one
    // column, silently, from the header down.
    let measurement = Measurement::new(
        vec![Channel {
            name: "Ch1\traw".to_owned(),
            unit: Unit::Volt,
            samples: Samples::I16(vec![1]),
            transform: Transform::IDENTITY,
            uncertainty: None,
        }],
        Vec::new(),
    );

    let refused = sample_table(&measurement, Values::Physical)
        .expect_err("a channel name carrying a tab cannot be written");
    assert_eq!(refused.what, "channel");
    assert_eq!(refused.name, "Ch1\traw");
    assert_eq!(refused.character, "a tab");

    // Both writers refuse the same names, so a measurement that writes one
    // output writes both.
    assert!(metadata_document(&measurement).is_err());

    // And the sentence a person reads names no Rust type.
    let sentence = refused.to_string();
    for name in ["Unwritable", "Values", "Measurement", "Err"] {
        assert!(!sentence.contains(name), "{sentence}");
    }
}

#[test]
fn the_metadata_document_is_the_bytes_this_test_writes_out() {
    // Everything that is not samples, indented for a person and splittable on
    // the first colon and space for a pipeline. The uncertainty that was not
    // stated says so rather than being written as zero.
    let document = metadata_document(&two_channels()).expect("these names are writable");
    assert_eq!(
        document,
        "axes:\n\
         \x20 - name: time\n\
         \x20   unit: s\n\
         \x20   positions: 3\n\
         \x20   shape: regular\n\
         \x20   start: 0\n\
         \x20   step: 4e-7\n\
         channels:\n\
         \x20 - name: Ch1\n\
         \x20   unit: V\n\
         \x20   samples: 3\n\
         \x20   stored width in bits: 16\n\
         \x20   transform:\n\
         \x20     scale: 3.0517578125e-5\n\
         \x20     offset: 0\n\
         \x20   uncertainty:\n\
         \x20     kind: relative, as a fraction of the reading\n\
         \x20     amount: 0.01\n\
         \x20 - name: Ch2\n\
         \x20   unit: V\n\
         \x20   samples: 3\n\
         \x20   stored width in bits: 16\n\
         \x20   transform:\n\
         \x20     scale: 1\n\
         \x20     offset: 0.5\n\
         \x20   uncertainty: not stated by the file\n\
         instrument:\n\
         \x20 not identified by the file\n\
         provenance:\n\
         \x20 none: this measurement did not come through the read path\n"
    );
}
