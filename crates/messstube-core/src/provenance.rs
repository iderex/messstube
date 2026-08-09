//! The provenance block, from #36, and the instrument identification a reader
//! is the only source of.
//!
//! BUILT ONCE BY THE LIBRARY AND NEVER BY A READER. That is the whole point of
//! the type living here rather than being a struct each reader fills in.
//! [`Provenance`] carries no public constructor and is `#[non_exhaustive]`, so
//! a reader crate cannot make one, and [`Measurement`](crate::measurement::Measurement)
//! holds its own behind a private field, so a reader cannot put one there
//! either. What a reader supplies is [`Instrument`], and nothing else about the
//! block depends on the reader remembering anything.
//!
//! WHAT IT DELIBERATELY DOES NOT CARRY. No conversion timestamp, no hostname and
//! no account name. The reasons are recorded in
//! `docs/decisions/0004-what-a-read-produces.md` rather than only here: the time
//! a conversion ran is not a property of the measurement, it makes every output
//! non-reproducible for no gain, and a machine name and an account name are
//! personal data that would be written into every file an operator shares.
//!
//! An operator who wants a conversion time records one. The library will not add
//! it quietly, and the absence is what makes
//! `two_reads_of_one_input_produce_the_same_provenance` in
//! `crates/messstube-core/tests/provenance.rs` possible at all.

use crate::hash::ContentHash;
use crate::reader::Maturity;

/// What the file itself said about the instrument that wrote it.
///
/// Every field is absent rather than guessed. A format that carries no serial
/// number produces a `None`, and a reader that fills in the model name it
/// believes the format implies has invented a traceability claim, which is the
/// same defect as an invented uncertainty in
/// `docs/decisions/0004-what-a-read-produces.md`.
///
/// This is the one part of the provenance block a reader supplies, and it is
/// supplied by putting it on the measurement.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instrument {
    /// Who made it, as the file spells it.
    pub manufacturer: Option<String>,
    /// The model, as the file spells it.
    pub model: Option<String>,
    /// The serial number, as the file spells it. This is what turns a converted
    /// table back into a measurement somebody can trace to a machine in a room.
    pub serial: Option<String>,
    /// The firmware or software version that wrote the file, where it says.
    pub firmware: Option<String>,
}

impl Instrument {
    /// Whether the file said nothing at all about the instrument.
    ///
    /// The provenance block records an absent instrument rather than an empty
    /// one, so that "the format carries no identification" and "the reader did
    /// not look" are not written down the same way.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.manufacturer.is_none()
            && self.model.is_none()
            && self.serial.is_none()
            && self.firmware.is_none()
    }
}

/// Where a measurement came from and what produced it.
///
/// Attached by the read path in [`read_with`](crate::read::read_with) and by
/// nothing else. Every field is readable; none is writable from outside this
/// crate and there is no public way to build one, which is what makes the block
/// a statement by the library rather than a claim by a reader.
///
/// A reader crate cannot construct one:
///
/// ```compile_fail
/// use messstube_core::hash::{ContentHash, HashAlgorithm};
/// use messstube_core::provenance::Provenance;
/// use messstube_core::reader::Maturity;
///
/// let forged = Provenance {
///     input: "not mine to write".to_owned(),
///     length: 0,
///     hash: ContentHash { algorithm: HashAlgorithm::Sha256, digest: String::new() },
///     reader: "any".to_owned(),
///     maturity: Maturity::Corroborated,
///     library_version: "99.0.0".to_owned(),
///     instrument: None,
/// };
/// ```
///
/// Reading one is ordinary:
///
/// ```
/// use messstube_core::provenance::Provenance;
///
/// fn who_read_it(provenance: &Provenance) -> &str {
///     &provenance.reader
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Provenance {
    /// The input as the caller named it. Whatever the caller passed in, not a
    /// path this library resolved, because the library never opened anything.
    pub input: String,
    /// How many bytes were read from that input, counted over exactly the bytes
    /// that were hashed.
    pub length: u64,
    /// The content hash of those bytes, with its algorithm beside it.
    pub hash: ContentHash,
    /// The stable identifier of the reader that produced the measurement.
    pub reader: String,
    /// What evidence stood behind that reader when the file was converted. It
    /// is recorded here because it moves over time, and somebody holding an old
    /// output needs to know what it meant then.
    pub maturity: Maturity,
    /// The version of this library.
    pub library_version: String,
    /// What the file said about the instrument, where it said anything.
    pub instrument: Option<Instrument>,
}

impl Provenance {
    /// Build the block. Inside this crate only, and called from the read path.
    pub(crate) fn new(
        input: String,
        length: u64,
        hash: ContentHash,
        reader: String,
        maturity: Maturity,
        instrument: Option<Instrument>,
    ) -> Self {
        Provenance {
            input,
            length,
            hash,
            reader,
            maturity,
            // From the manifest at compile time rather than from anything the
            // running process can see, so it is the version that did the
            // reading and not the version installed beside it.
            library_version: env!("CARGO_PKG_VERSION").to_owned(),
            instrument: instrument.filter(|found| !found.is_empty()),
        }
    }

    /// The block as ordered name and value pairs.
    ///
    /// One place decides the order and the spelling, so the tool, the metadata
    /// document in #38 and a test comparing two runs are all looking at the same
    /// bytes. An absent field is absent from this list rather than present and
    /// empty, because an empty serial number and a format that carries none are
    /// different statements.
    #[must_use]
    pub fn fields(&self) -> Vec<(String, String)> {
        let mut written = vec![
            ("input".to_owned(), self.input.clone()),
            ("input length in bytes".to_owned(), self.length.to_string()),
            (
                "content hash algorithm".to_owned(),
                self.hash.algorithm.to_string(),
            ),
            ("content hash".to_owned(), self.hash.digest.clone()),
            ("reader".to_owned(), self.reader.clone()),
            ("reader maturity".to_owned(), self.maturity.to_string()),
            ("library version".to_owned(), self.library_version.clone()),
        ];

        if let Some(instrument) = &self.instrument {
            let named = [
                ("instrument manufacturer", &instrument.manufacturer),
                ("instrument model", &instrument.model),
                ("instrument serial number", &instrument.serial),
                ("instrument firmware", &instrument.firmware),
            ];
            for (name, value) in named {
                if let Some(value) = value {
                    written.push((name.to_owned(), value.clone()));
                }
            }
        }

        written
    }
}

#[cfg(test)]
mod tests {
    //! What this module decides on its own: which fields the block carries,
    //! which it refuses to carry, and how an absent one is written down.
    //!
    //! That the block is attached by the read path and not by a reader is
    //! proved in `crates/messstube-core/tests/provenance.rs`, because it is a
    //! property of the read path rather than of this type.

    use super::{Instrument, Provenance};
    use crate::hash::{ContentHash, HashAlgorithm};
    use crate::reader::Maturity;

    fn block(instrument: Option<Instrument>) -> Provenance {
        Provenance::new(
            "measurements/run-14.isf".to_owned(),
            2048,
            ContentHash {
                algorithm: HashAlgorithm::Sha256,
                digest: "e3b0c44298fc1c14".to_owned(),
            },
            "tektronix-isf".to_owned(),
            Maturity::Sketched,
            instrument,
        )
    }

    #[test]
    fn the_block_carries_no_time_no_machine_and_no_account() {
        // THE WHOLE FIELD LIST, not a search for forbidden words. A word search
        // was what stood here first, and a field called `converted at` carrying
        // a timestamp walked straight through it: none of the words it looked
        // for appear in either the name or an ISO 8601 value. What refuses a
        // field nobody thought of is pinning the set, so anything added has to
        // be added here too and argued for at that moment.
        let named: Vec<String> = block(None)
            .fields()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(
            named,
            vec![
                "input".to_owned(),
                "input length in bytes".to_owned(),
                "content hash algorithm".to_owned(),
                "content hash".to_owned(),
                "reader".to_owned(),
                "reader maturity".to_owned(),
                "library version".to_owned(),
            ],
            "a field was added to or removed from the provenance block"
        );

        // And the second net, which catches a value that carries what the names
        // do not. It is the weaker of the two and it is kept because the two
        // fail on different mistakes.
        let mut rendered = String::new();
        for (name, value) in block(None).fields() {
            rendered.push_str(&name);
            rendered.push('=');
            rendered.push_str(&value);
            rendered.push('\n');
        }

        for forbidden in ["time", "date", "host", "machine", "user", "account"] {
            assert!(
                !rendered.to_lowercase().contains(forbidden),
                "the provenance block names {forbidden}: {rendered}"
            );
        }
    }

    #[test]
    fn an_instrument_the_file_did_not_describe_is_absent_rather_than_empty() {
        // A reader that found nothing hands back an empty value or nothing at
        // all, and both mean the same thing, so both have to be written down
        // the same way.
        assert_eq!(block(None).instrument, None);
        assert_eq!(block(Some(Instrument::default())).instrument, None);

        let found = Instrument {
            model: Some("TDS 3054".to_owned()),
            ..Instrument::default()
        };
        assert_eq!(block(Some(found.clone())).instrument, Some(found));
    }

    #[test]
    fn only_the_fields_the_file_carried_are_written_down() {
        let named: Vec<String> = block(Some(Instrument {
            serial: Some("C010203".to_owned()),
            ..Instrument::default()
        }))
        .fields()
        .into_iter()
        .map(|(name, _)| name)
        .collect();

        assert!(named.contains(&"instrument serial number".to_owned()));
        assert!(!named.contains(&"instrument model".to_owned()));
        assert!(!named.contains(&"instrument manufacturer".to_owned()));

        // And the fields that are always there, in the order this module fixes.
        assert_eq!(
            named.first().map(String::as_str),
            Some("input"),
            "the input is what a person looks for first"
        );
        assert!(named.contains(&"content hash algorithm".to_owned()));
    }

    #[test]
    fn the_version_is_the_one_that_was_compiled_in() {
        // Not a version read out of the environment at run time, which would be
        // the version of something else on the machine.
        assert_eq!(block(None).library_version, env!("CARGO_PKG_VERSION"));
    }
}
