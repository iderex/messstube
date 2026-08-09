//! The read path, from #36. What a caller uses instead of calling a reader.
//!
//! WHY THERE IS A LAYER HERE AT ALL. The provenance block has to be built once,
//! by the library, for every measurement. A block each reader assembled would
//! be forgotten by the reader written in a hurry and would differ in spelling
//! between the ones that remembered. So [`read_with`] hashes the input, counts
//! it, calls the reader, and attaches the block itself. A reader supplies the
//! instrument identification it recovered and nothing else.
//!
//! THE INPUT IS HASHED BEFORE THE READER SEES IT, over the whole source, and the
//! source is wound back to where it started before the reader is handed it. The
//! digest is therefore of the bytes the caller offered rather than of the bytes
//! the reader happened to consume, which is what makes it check against the file
//! on disk with an ordinary checksum tool.
//!
//! CHOOSING THE READER IS NOT HERE. This function is told which reader to use.
//! Identification over a bounded prefix, and the refusal when two readers claim
//! one file, is #33, and it will call this once it can name a reader.

use crate::error::{ReadError, ReadOptions, ReadOutcome};
use crate::hash::digest_of;
use crate::provenance::Provenance;
use crate::reader::{Reader, Source};
use core::fmt;
use std::io::SeekFrom;

/// Why a read did not get as far as a reader's verdict.
///
/// TWO VARIANTS, AND THE SECOND IS NOT A THIRD ERROR KIND. `docs/decisions/0006-errors-and-partial-reads.md`
/// fixes what a READER may say about bytes at two kinds, and [`ReadError`] is
/// still exactly those two. This type sits one layer above and holds the
/// question that layer asks and a reader never does: whether the bytes could be
/// obtained at all. A disk that stopped answering is not a damaged file, and
/// reporting it as one would send somebody looking for corruption in a file that
/// is intact.
#[derive(Debug)]
pub enum ReadPathError {
    /// The input could not be read to the end, so there is nothing to hash and
    /// nothing to hand a reader.
    Unreadable {
        /// The input as the caller named it.
        input: String,
        /// What the source said, in its own words.
        detail: String,
    },
    /// The reader was reached and gave its verdict.
    Refused(ReadError),
}

impl fmt::Display for ReadPathError {
    /// Constraint 3 of `docs/decisions/0002-product-surface.md`: no Rust
    /// vocabulary reaches a person.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadPathError::Unreadable { input, detail } => {
                write!(formatter, "{input} could not be read: {detail}")
            }
            ReadPathError::Refused(refusal) => refusal.fmt(formatter),
        }
    }
}

impl core::error::Error for ReadPathError {}

impl From<ReadError> for ReadPathError {
    fn from(refusal: ReadError) -> Self {
        ReadPathError::Refused(refusal)
    }
}

/// Read an input with a named reader, and attach the provenance block.
///
/// `input` is the name the caller uses for the bytes, and it is written into the
/// block unchanged. This library opens nothing, so it has no path of its own to
/// record and no way to check the one it is given.
///
/// # Errors
///
/// [`ReadPathError::Unreadable`] where the source could not be read to the end
/// or could not be wound back, and [`ReadPathError::Refused`] carrying the
/// reader's own verdict otherwise.
pub fn read_with(
    reader: &dyn Reader,
    input: &str,
    source: &mut dyn Source,
    options: ReadOptions,
) -> Result<ReadOutcome, ReadPathError> {
    let unreadable = |detail: String| ReadPathError::Unreadable {
        input: input.to_owned(),
        detail,
    };

    // From the start of the source and not from wherever the caller left it, so
    // that the digest is over the whole input. A caller who wound the source
    // forward gets the hash of the file, not of the tail of it.
    source
        .seek(SeekFrom::Start(0))
        .map_err(|failure| unreadable(failure.to_string()))?;

    let (hash, length) = digest_of(source).map_err(|failure| unreadable(failure.to_string()))?;

    source
        .seek(SeekFrom::Start(0))
        .map_err(|failure| unreadable(failure.to_string()))?;

    let mut outcome = reader.read(source, options)?;

    // Taken off the measurement rather than asked of the reader through a second
    // method, so that a reader which found nothing has already said so by
    // leaving the field alone.
    let instrument = outcome.measurement.instrument.clone();
    outcome.measurement.attach(Provenance::new(
        input.to_owned(),
        length,
        hash,
        reader.id(),
        reader.maturity(),
        instrument,
    ));

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    //! What this module decides: that the block is attached by this function,
    //! over the whole input, whatever the reader did with the source.
    //!
    //! The fixture reader below is deliberately a reader that seeks around and
    //! stops early, because a reader that read tidily from the start would let a
    //! read path that hashed only what the reader consumed pass.

    // Turned off for test code only, for the reason given in `hash.rs`: a test
    // that reached a branch it has just proved unreachable has to stop rather
    // than go on and report a pass.
    #![allow(clippy::unreachable)]

    use super::{ReadPathError, read_with};
    use crate::error::{ReadError, ReadOptions, ReadOutcome};
    use crate::measurement::Measurement;
    use crate::provenance::Instrument;
    use crate::reader::{Family, Maturity, Reader, Source};
    use std::io::{Read as _, SeekFrom};

    /// A reader that consumes four bytes from the middle and reports the
    /// instrument it claims to have found there.
    struct Untidy {
        instrument: Option<Instrument>,
    }

    impl Reader for Untidy {
        fn id(&self) -> String {
            "untidy".to_owned()
        }
        fn name(&self) -> String {
            "the untidy fixture".to_owned()
        }
        fn family(&self) -> Family {
            Family::Oscilloscope
        }
        fn maturity(&self) -> Maturity {
            Maturity::Sketched
        }
        fn recognises(&self, _prefix: &[u8]) -> bool {
            true
        }
        fn read(
            &self,
            source: &mut dyn Source,
            _options: ReadOptions,
        ) -> Result<ReadOutcome, ReadError> {
            let mut four = [0_u8; 4];
            if source.seek(SeekFrom::Start(2)).is_err() || source.read_exact(&mut four).is_err() {
                return Err(ReadError::Damaged {
                    reader: self.id(),
                    offset: 2,
                    expected: "4 bytes".to_owned(),
                    found: "end of file".to_owned(),
                });
            }
            let mut measurement = Measurement::new(Vec::new(), Vec::new());
            measurement.instrument.clone_from(&self.instrument);
            Ok(ReadOutcome::complete(measurement))
        }
    }

    /// A reader that declines everything, so the refusal path is about the read
    /// path rather than about parsing.
    struct Declining;

    impl Reader for Declining {
        fn id(&self) -> String {
            "declining".to_owned()
        }
        fn name(&self) -> String {
            "the declining fixture".to_owned()
        }
        fn family(&self) -> Family {
            Family::Hall
        }
        fn maturity(&self) -> Maturity {
            Maturity::Sketched
        }
        fn recognises(&self, _prefix: &[u8]) -> bool {
            false
        }
        fn read(
            &self,
            _source: &mut dyn Source,
            _options: ReadOptions,
        ) -> Result<ReadOutcome, ReadError> {
            Err(ReadError::NotThisFormat { reader: self.id() })
        }
    }

    #[test]
    fn the_digest_is_of_the_whole_input_and_not_of_what_the_reader_read() {
        // Sixteen bytes, of which the reader consumes four from the middle.
        // What is asserted is the length and that the digest is the one the
        // whole input has on its own, so a read path hashing only the four bytes
        // the reader took would red here. That the digest is SHA-256 rather than
        // something that merely behaves like it is carried by the published
        // vectors in `hash.rs`, which is a different question and a different
        // test.
        let bytes = b"MSTB0123456789ab".to_vec();
        let mut source = std::io::Cursor::new(bytes.clone());
        let reader = Untidy { instrument: None };

        let Ok(outcome) = read_with(&reader, "in memory", &mut source, ReadOptions::default())
        else {
            unreachable!("the untidy fixture reads this input")
        };
        let Some(provenance) = outcome.measurement.provenance() else {
            unreachable!("the read path attaches a block to every measurement")
        };

        assert_eq!(provenance.length, 16);
        assert_eq!(provenance.input, "in memory");
        assert_eq!(provenance.reader, "untidy");

        // The same bytes hashed on their own, through the same function, so the
        // assertion is that the reader's seeking changed nothing rather than
        // that both paths are wrong in the same way. That second bound is what
        // the published vectors in `hash.rs` carry.
        let mut plain = std::io::Cursor::new(bytes);
        let Ok((direct, direct_length)) = crate::hash::digest_of(&mut plain) else {
            unreachable!("a cursor over bytes in memory cannot fail to read")
        };
        assert_eq!(provenance.hash, direct);
        assert_eq!(provenance.length, direct_length);
    }

    #[test]
    fn a_source_the_caller_wound_forward_is_hashed_from_its_start() {
        // The failure this prevents: a caller that already peeked at the header
        // gets the digest of the remainder, which matches nothing on disk.
        let mut source = std::io::Cursor::new(b"MSTB0123456789ab".to_vec());
        let mut skip = [0_u8; 6];
        assert!(source.read_exact(&mut skip).is_ok());

        let reader = Untidy { instrument: None };
        let Ok(outcome) = read_with(&reader, "wound on", &mut source, ReadOptions::default())
        else {
            unreachable!("the untidy fixture reads this input")
        };
        let Some(provenance) = outcome.measurement.provenance() else {
            unreachable!("the read path attaches a block to every measurement")
        };
        assert_eq!(provenance.length, 16, "the digest was taken of the tail");
    }

    #[test]
    fn the_instrument_is_the_readers_and_the_rest_of_the_block_is_not() {
        let found = Instrument {
            model: Some("TDS 3054".to_owned()),
            ..Instrument::default()
        };
        let reader = Untidy {
            instrument: Some(found.clone()),
        };
        let mut source = std::io::Cursor::new(b"MSTB0123456789ab".to_vec());

        let Ok(outcome) = read_with(&reader, "in memory", &mut source, ReadOptions::default())
        else {
            unreachable!("the untidy fixture reads this input")
        };
        let Some(provenance) = outcome.measurement.provenance() else {
            unreachable!("the read path attaches a block to every measurement")
        };

        assert_eq!(provenance.instrument, Some(found));
        // And the parts the reader had no say in came from the read path.
        assert_eq!(provenance.maturity, Maturity::Sketched);
        assert_eq!(provenance.library_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn a_reader_that_refuses_produces_no_block_and_the_refusal_is_unchanged() {
        // The refusal is passed through rather than restated, so a caller
        // branching on the two kinds sees exactly what the reader said.
        let mut source = std::io::Cursor::new(b"MSTB0123456789ab".to_vec());
        let refused = read_with(&Declining, "in memory", &mut source, ReadOptions::default());

        match refused {
            Err(ReadPathError::Refused(ReadError::NotThisFormat { reader })) => {
                assert_eq!(reader, "declining");
            }
            other => unreachable!("the declining fixture refuses every input: {other:?}"),
        }
    }

    #[test]
    fn a_source_that_cannot_be_read_is_not_reported_as_a_damaged_file() {
        // The distinction the second variant exists for. A caller told that an
        // intact file is damaged goes looking for corruption that is not there.
        let failing = ReadPathError::Unreadable {
            input: "over the wire".to_owned(),
            detail: "the device is not ready".to_owned(),
        };
        let sentence = failing.to_string();
        assert!(sentence.contains("over the wire"), "{sentence}");
        assert!(sentence.contains("could not be read"), "{sentence}");
        for name in ["Damaged", "NotThisFormat", "Unreadable", "ReadPathError"] {
            assert!(
                !sentence.contains(name),
                "{sentence} names the Rust identifier {name}"
            );
        }
    }
}
