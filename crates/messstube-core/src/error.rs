//! What a read says when it does not return a whole measurement, from #34,
//! implementing `docs/decisions/0006-errors-and-partial-reads.md`.
//!
//! TWO KINDS, AND THEY ARE NEVER COLLAPSED. [`ReadError::NotThisFormat`] means
//! the reader did not recognise the file and another reader should be tried.
//! [`ReadError::Damaged`] means this reader recognised the file, something
//! inside it is wrong, and no other reader will do better. A caller acts on that
//! distinction in opposite directions, which is why it is the shape of the type
//! rather than a field inside one kind. A flat list of variants naming every way
//! a file can be wrong puts the two statements side by side and lets a caller
//! read one as the other.
//!
//! THE OFFSET IS ABSOLUTE IN THE FILE. Not relative to whatever structure the
//! reader was inside when it stopped. The person receiving the message has a hex
//! editor and not the reader's source, so an offset they cannot type into it is
//! an offset that does not help them. `an_offset_is_absolute_in_the_file` in
//! `crates/messstube-core/tests/error_model.rs` is where that is asserted
//! against a fixture whose relative and absolute offsets differ.
//!
//! THE PARTIAL READ IS AN OPTION AND IS NEVER THE DEFAULT. It is carried by
//! [`ReadOptions`], which a read takes instead of growing an argument, so that
//! the next option after this one is not a change to every reader. A caller who
//! did not ask for a partial result will not check whether they got one, so a
//! caller who does not ask gets a refusal.
//!
//! NOTHING IS SYNTHESISED. A channel that was half read does not come back
//! truncated and silently. It is absent from [`ReadOutcome::measurement`] and
//! present in [`ReadOutcome::losses`], with the sample index it ended at. A zero
//! is indistinguishable from a measurement of zero, and this field is full of
//! measurements that are legitimately zero.
//!
//! This module also holds [`ReadOutcome`], which is the success side rather than
//! the error side, because 0006 decides the refusal and the partial result
//! together and splitting them across two modules would put half of one decision
//! in each.
//!
//! NOTHING HERE READS OR WRITES ANYTHING, the same property
//! `crates/messstube-core/src/measurement.rs` is checked against. A type
//! describing a failure that reached outside itself to describe it would be a
//! reader.

use crate::measurement::Measurement;
use core::fmt;

/// Why a read did not return a whole measurement.
///
/// One type for the whole library rather than one per reader, because the caller
/// has to be able to branch on the distinction without knowing which reader ran.
///
/// The identifier in both kinds is the reader's stable one. #32 is where a
/// reader declares it; until that interface exists it is the plain owned string
/// a reader would put there, which is also what constraint 1 of
/// `docs/decisions/0002-product-surface.md` asks of every value crossing this
/// interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The reader did not recognise the file. Try another reader.
    ///
    /// It carries the reader that declined and nothing else. There is no offset
    /// here on purpose: a reader that declined has not established where in the
    /// file anything is, and an offset would invite a caller to believe it had.
    NotThisFormat {
        /// The stable identifier of the reader that declined.
        reader: String,
    },
    /// The reader recognised the file and something inside it is wrong. No other
    /// reader will do better.
    Damaged {
        /// The stable identifier of the reader that recognised the file.
        reader: String,
        /// Where the reader stopped, counted from the start of the input.
        ///
        /// `u64` rather than `usize`, so that the offset a 64-bit machine
        /// reports for a large file is the same number a 32-bit one reports.
        offset: u64,
        /// What the reader required at that offset, in the file's own
        /// vocabulary.
        expected: String,
        /// What was there instead.
        found: String,
    },
}

impl fmt::Display for ReadError {
    /// The sentence a tool prints. Constraint 3 of
    /// `docs/decisions/0002-product-surface.md` is that an error is describable
    /// without Rust vocabulary, so no variant name, no type name and no debug
    /// formatting appears in what a caller reads.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::NotThisFormat { reader } => {
                write!(
                    formatter,
                    "the {reader} reader does not recognise this file"
                )
            }
            ReadError::Damaged {
                reader,
                offset,
                expected,
                found,
            } => write!(
                formatter,
                "the {reader} reader stopped at byte {offset} of this file: \
                 expected {expected}, found {found}"
            ),
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, and the difference is
// not cosmetic. It is what lets this crate be built without the standard library
// later without the error type being the thing that stops it, and it keeps this
// module's imports off the four names a type in this set may not reach.
impl core::error::Error for ReadError {}

/// What a caller asks of a read, as a value rather than as arguments.
///
/// A read takes one of these instead of growing a parameter, so that the option
/// after this one is an addition here rather than a change to every reader that
/// exists by then. #32 fixes that shape for the interface; this type is what it
/// takes.
///
/// [`ReadOptions::default`] is the refusing behaviour: partial reads off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReadOptions {
    /// Off by default, which is the whole of the opt-in. A `bool` field named
    /// for what turning it on does, rather than an enum, because there are two
    /// states and 0006 does not admit a third.
    partial_reads: bool,
}

impl ReadOptions {
    /// Ask for, or stop asking for, a partial read.
    ///
    /// Consumes and returns the value so that a caller writes the whole request
    /// in one expression and cannot leave a half-built options value in scope.
    #[must_use]
    pub const fn partial_reads(mut self, requested: bool) -> Self {
        self.partial_reads = requested;
        self
    }

    /// Whether this caller asked for a partial read.
    ///
    /// A reader consults this and nothing else before returning losses instead
    /// of refusing.
    #[must_use]
    pub const fn partial_reads_requested(&self) -> bool {
        self.partial_reads
    }
}

/// What a read returned when it returned something.
///
/// The losses are empty for every read that was not asked for a partial result,
/// because such a read either produced the whole measurement or refused. So a
/// caller who never sets the option never has to check the list, and a caller
/// who does has one place to look.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadOutcome {
    /// What was recovered. Every channel in it was recovered in full.
    pub measurement: Measurement,
    /// What was not recovered, in the order the reader met it. Empty where
    /// nothing was lost.
    pub losses: Vec<Loss>,
}

impl ReadOutcome {
    /// A whole measurement, with nothing lost.
    ///
    /// This is what a read that was not asked for a partial result returns when
    /// it succeeds, and writing it through one function is what keeps a reader
    /// from spelling the empty list differently each time.
    #[must_use]
    pub const fn complete(measurement: Measurement) -> Self {
        ReadOutcome {
            measurement,
            losses: Vec::new(),
        }
    }

    /// Whether anything was lost.
    ///
    /// The question a caller asks before treating the measurement as the file.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.losses.is_empty()
    }
}

/// One thing a partial read did not recover.
///
/// It carries where and why, and where the loss was of a channel it carries
/// which channel and the sample index the channel ended at. That index is the
/// alternative to returning the channel short: a caller can see that eleven of
/// the promised two thousand samples arrived, which a truncated vector of eleven
/// samples does not say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loss {
    /// Where the reader stopped recovering, counted from the start of the input.
    /// Absolute, for the same reason [`ReadError::Damaged`] carries an absolute
    /// one.
    pub offset: u64,
    /// What went wrong there, in the same words the refusing path would have
    /// used.
    pub reason: String,
    /// The channel that was lost, where the loss was of a channel. `None` where
    /// what was lost was not one, which is where a reader loses a trailer or a
    /// metadata block rather than data.
    pub channel: Option<String>,
    /// The sample index the channel's data ended at, where the loss was of a
    /// channel. `None` for the same case as above.
    pub ended_at_sample: Option<usize>,
}

#[cfg(test)]
mod tests {
    //! Unit tests for the parts of this module that are decided here rather
    //! than by a reader: what a caller reads off an error, and what the options
    //! value does when nobody sets anything on it.
    //!
    //! The two kinds themselves are proved against fixtures in
    //! `crates/messstube-core/tests/error_model.rs`, because what has to be
    //! shown there is that a reader arrives at different kinds from different
    //! bytes, and that is a thing a caller does rather than a property of a
    //! type.

    use super::{Loss, ReadError, ReadOptions, ReadOutcome};
    use crate::measurement::Measurement;

    #[test]
    fn a_partial_read_is_off_until_somebody_asks_for_it() {
        // The whole of the opt-in, and it is asserted rather than trusted to the
        // derive: 0006 refuses partial reads by default, and the default is what
        // every caller who has not read 0006 will get.
        assert!(!ReadOptions::default().partial_reads_requested());

        let asked = ReadOptions::default().partial_reads(true);
        assert!(asked.partial_reads_requested());

        // And it goes back off, so a caller building options in a loop cannot
        // leave the option latched on for a later file.
        assert!(!asked.partial_reads(false).partial_reads_requested());
    }

    #[test]
    fn what_a_caller_reads_off_an_error_names_no_rust_type() {
        // Constraint 3 of `docs/decisions/0002-product-surface.md`, asserted on
        // the sentence a tool would print. What is checked is the absence of
        // this repository's own Rust vocabulary from it, because that is the
        // vocabulary a binding cannot carry across.
        let declined = ReadError::NotThisFormat {
            reader: "tektronix-isf".to_owned(),
        }
        .to_string();
        let damaged = ReadError::Damaged {
            reader: "tektronix-isf".to_owned(),
            offset: 23,
            expected: "2 bytes of sample data".to_owned(),
            found: "end of file".to_owned(),
        }
        .to_string();

        for sentence in [&declined, &damaged] {
            for name in ["NotThisFormat", "Damaged", "ReadError", "None", "Some"] {
                assert!(
                    !sentence.contains(name),
                    "{sentence} names the Rust identifier {name}"
                );
            }
        }

        // The offset is in the sentence as a number somebody can type into a hex
        // editor, rather than described.
        assert!(damaged.contains("byte 23"), "{damaged}");
        assert!(declined.contains("tektronix-isf"), "{declined}");
    }

    #[test]
    fn a_complete_outcome_has_no_losses_and_a_lossy_one_is_not_complete() {
        let empty = Measurement {
            channels: Vec::new(),
            axes: Vec::new(),
        };

        let whole = ReadOutcome::complete(empty.clone());
        assert!(whole.is_complete());
        assert!(whole.losses.is_empty());

        let lossy = ReadOutcome {
            measurement: empty,
            losses: vec![Loss {
                offset: 23,
                reason: "the file ends inside the samples".to_owned(),
                channel: Some("Ch2".to_owned()),
                ended_at_sample: Some(1),
            }],
        };
        assert!(!lossy.is_complete());
    }

    #[test]
    fn this_module_reaches_none_of_the_four_doors_to_the_outside() {
        // The same property `measurement.rs` is held to, restated over this
        // file because an error type is where a reach outside would be easiest
        // to excuse: reading the file again to say what was in it.
        //
        // The names are assembled rather than written out, so that this test is
        // not a match against itself.
        const DOORS: [&str; 4] = ["fs", "io", "net", "path"];

        let source = include_str!("error.rs");
        for door in DOORS {
            let needle = format!("std::{door}");
            assert!(
                !source.contains(&needle),
                "this module names {needle}, so an error reaches outside itself"
            );
        }
    }
}
