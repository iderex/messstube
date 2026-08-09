//! Which reader claims a file, decided on a bounded prefix, from #33 and
//! implementing `docs/decisions/0005-identification.md`.
//!
//! THREE ANSWERS AND TWO OF THEM ARE REFUSALS. Exactly one reader claims the
//! file, several do, or none does. The second and third need different actions
//! from the person reading them, so they are different variants carrying
//! different words rather than one failure with a message inside it.
//!
//! THE AMBIGUITY NAMES EVERY CLAIMANT. That is the message that turns a user
//! report into a fix: the repair is always to tighten one of the named
//! predicates, and without the names nobody can tell which. It is also why
//! every predicate is run rather than the first match being taken. There is no
//! early exit here and there cannot be one, because a claimant that was never
//! asked is a collision nobody sees.
//!
//! THE PREFIX IS BOUNDED AND FIXED FOR EVERY READER.
//! [`RECOGNITION_PREFIX`](crate::reader::RECOGNITION_PREFIX) bytes, read once
//! and offered to all of them. A reader needing more than that to recognise a
//! file is a reader whose format has no reliable signature near the front,
//! which is a real situation and is handled by admitting it rather than by
//! reading half the file at identification time.
//!
//! A FILE SHORTER THAN THE PREFIX IS SHOWN WHAT THERE IS. The bound is a
//! maximum and never a requirement, and a predicate asserting it was handed the
//! whole bound would decline every short file.
//!
//! THE NAME ORDERS AND NEVER DECIDES. A file called `sweep.dat` whose bytes
//! match one reader is identified by the bytes; a file called `sweep.dat` whose
//! bytes match nothing is unrecognised, not assumed. What the extension is
//! allowed to do is put the likely reader at the front of the list a person
//! reads, and [`the_extension_orders_the_answer_and_never_decides_it`] is where
//! that boundary is held.
//!
//! NOTHING HERE OPENS ANYTHING. [`prefix_of`] is handed a source the caller
//! already has, the same way a reader is, and it winds it back afterwards so
//! that the caller can hand the same source to the read path.

use crate::reader::{RECOGNITION_PREFIX, ReaderInfo, Registry};
use core::fmt;
use std::io::{Read, Seek, SeekFrom};

/// What identification concluded.
///
/// Owned data and no lifetime, which is what
/// `docs/decisions/0002-product-surface.md` asks of a value a caller receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identification {
    /// Exactly one reader claimed the file. This is the answer; the other two
    /// are refusals.
    Recognised(ReaderInfo),
    /// More than one reader claimed the file, and here is every one of them.
    ///
    /// A defect in this repository rather than in the file. Two predicates
    /// overlap and one of them has to be tightened, and the list is what says
    /// which two.
    Ambiguous(Vec<ReaderInfo>),
    /// No reader claimed the file.
    ///
    /// Not a damaged file and not a defect. It is a format nothing here reads
    /// yet, which is a different thing from a file that is broken, and
    /// `docs/decisions/0010-versioning-and-stability.md` gives the two
    /// different exit codes for exactly that reason.
    Unrecognised,
}

impl fmt::Display for Identification {
    /// The sentence a tool prints, describable without Rust vocabulary, which
    /// is constraint 3 of `docs/decisions/0002-product-surface.md`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Identification::Recognised(info) => {
                write!(formatter, "read by {} ({})", info.name, info.id)
            }
            Identification::Ambiguous(claimants) => {
                let named: Vec<&str> = claimants.iter().map(|info| info.id.as_str()).collect();
                write!(
                    formatter,
                    "claimed by more than one reader, which is a defect in this software \
                     rather than in the file: {}. One of those recognition rules is too \
                     broad and has to be narrowed.",
                    named.join(", ")
                )
            }
            Identification::Unrecognised => formatter.write_str(
                "not recognised by any reader compiled into this build. The file is not \
                 said to be damaged; nothing here reads its format.",
            ),
        }
    }
}

/// The bounded prefix of a source, wound back afterwards.
///
/// Reads at most [`RECOGNITION_PREFIX`](crate::reader::RECOGNITION_PREFIX)
/// bytes and returns what it got, which is fewer where the file is shorter. The
/// source is left where it was found, at the start, so the caller can hand the
/// same source to [`read_with`](crate::read::read_with) without knowing that
/// identification touched it.
///
/// # Errors
///
/// Whatever the source raised, on the seek or on the read. This function
/// decides nothing about a failure to read; that judgement belongs to the
/// caller.
pub fn prefix_of<S: Read + Seek + ?Sized>(source: &mut S) -> Result<Vec<u8>, std::io::Error> {
    source.seek(SeekFrom::Start(0))?;

    // A fixed buffer rather than a size taken from the file. The bound is this
    // repository's own constant, so nothing an input says can change how much
    // is read here, which is part one of
    // docs/decisions/0007-hostile-input-budget.md holding at the one place a
    // read happens before any reader has seen the bytes.
    let mut prefix = vec![0_u8; RECOGNITION_PREFIX];
    let mut filled = 0_usize;
    while filled < RECOGNITION_PREFIX {
        let into = prefix.get_mut(filled..).unwrap_or_default();
        let read = source.read(into)?;
        if read == 0 {
            break;
        }
        filled = filled.saturating_add(read);
    }
    prefix.truncate(filled);

    source.seek(SeekFrom::Start(0))?;
    Ok(prefix)
}

/// Which reader claims these bytes.
///
/// `name` is what the caller calls the input, and it is used for one thing: to
/// put the reader whose declared extensions match at the front of the answer.
/// It never adds a claimant, never removes one and never breaks a tie. Pass
/// `None` where there is no name, which is the case whenever the bytes did not
/// come from a file.
#[must_use]
pub fn identify(registry: Registry, prefix: &[u8], name: Option<&str>) -> Identification {
    // EVERY predicate, always. Stopping at the first claimant would make an
    // overlapping pair depend on link order and would report one reader for a
    // file two of them recognise.
    //
    // The prefix is bounded by `claimants`, which is the place bytes are handed
    // to a predicate, and it is not bounded a second time here. A second cut
    // would be a guard no test could tell from its absence, because the first
    // one holds whether or not it is there, and a guard nothing can distinguish
    // is a guard nobody knows is working.
    let mut claimants: Vec<ReaderInfo> = registry.claimants(prefix);

    if let Some(suffix) = name.and_then(extension_of) {
        // A stable partition rather than a sort: the readers whose extensions
        // match keep their order among themselves and so do the rest. A sort
        // would reorder readers that are equally likely, and the registry's own
        // order is the one thing here that is the same on every machine.
        let (likely, rest): (Vec<ReaderInfo>, Vec<ReaderInfo>) = claimants
            .into_iter()
            .partition(|info| info.extensions.contains(&suffix));
        claimants = likely;
        claimants.extend(rest);
    }

    match claimants.len() {
        0 => Identification::Unrecognised,
        1 => claimants
            .into_iter()
            .next()
            .map_or(Identification::Unrecognised, Identification::Recognised),
        _ => Identification::Ambiguous(claimants),
    }
}

/// The extension of a name, lower-cased, without its dot.
///
/// `None` where the name has no extension, which is common on instrument files
/// and is exactly the case in which nothing may be assumed.
fn extension_of(name: &str) -> Option<String> {
    let last = name.rsplit(['/', '\\']).next().unwrap_or(name);
    // `rsplit_once` rather than `split_once`, because `sweep.raw.dat` has the
    // extension `dat`. A leading dot is a hidden file and not an extension, so
    // a name that is nothing but an extension has none.
    let (stem, suffix) = last.rsplit_once('.')?;
    if stem.is_empty() || suffix.is_empty() {
        return None;
    }
    Some(suffix.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    //! Proved over fixture registries rather than over
    //! [`COMPILED_IN`](crate::reader::COMPILED_IN), which is empty. A test
    //! judging the registry this tree ships would report the state of the tree
    //! on the day it ran; what is proved here is what identification does with
    //! the readers it is given.

    // Turned off for test code only: a test whose precondition does not hold has
    // to stop loudly and say which precondition that was.
    #![allow(clippy::panic)]

    use super::{Identification, extension_of, identify, prefix_of};
    use crate::error::{ReadError, ReadOptions, ReadOutcome};
    use crate::reader::{Family, Maturity, RECOGNITION_PREFIX, Reader, Registry, Source};

    /// A reader that claims files starting with its own magic.
    struct Magic {
        id: &'static str,
        claims: &'static [u8],
        extensions: &'static [&'static str],
    }

    impl Reader for Magic {
        fn id(&self) -> String {
            self.id.to_owned()
        }
        fn name(&self) -> String {
            format!("the {} fixture", self.id)
        }
        fn family(&self) -> Family {
            Family::Oscilloscope
        }
        fn maturity(&self) -> Maturity {
            Maturity::Sketched
        }
        fn extensions(&self) -> Vec<String> {
            self.extensions
                .iter()
                .map(|suffix| (*suffix).to_owned())
                .collect()
        }
        fn recognises(&self, prefix: &[u8]) -> bool {
            // The bound is a maximum and never a requirement. A predicate
            // asserting it was shown the whole prefix would decline every file
            // shorter than the bound, which is most damaged ones.
            assert!(prefix.len() <= RECOGNITION_PREFIX);
            prefix.starts_with(self.claims)
        }
        fn read(
            &self,
            _source: &mut dyn Source,
            _options: ReadOptions,
        ) -> Result<ReadOutcome, ReadError> {
            Err(ReadError::NotThisFormat { reader: self.id() })
        }
    }

    const ALPHA: Magic = Magic {
        id: "alpha",
        claims: b"AL",
        extensions: &["alp"],
    };
    const BETA: Magic = Magic {
        id: "beta",
        claims: b"BE",
        extensions: &["dat"],
    };
    /// Overlaps ALPHA deliberately: a predicate one byte looser, which is the
    /// mistake that produces a real ambiguity.
    const LOOSE: Magic = Magic {
        id: "loose",
        claims: b"A",
        extensions: &["alp", "dat"],
    };

    const TWO: Registry = Registry::new(&[&ALPHA, &BETA]);
    const OVERLAPPING: Registry = Registry::new(&[&ALPHA, &LOOSE]);

    #[test]
    fn exactly_one_claimant_is_the_answer_and_the_other_two_are_refusals() {
        let recognised = identify(TWO, b"ALPHA-DATA", None);
        match recognised {
            Identification::Recognised(info) => assert_eq!(info.id, "alpha"),
            other => panic!("one claimant did not produce an answer: {other:?}"),
        }

        // Bytes no predicate claims. Not damaged and not ambiguous: a third
        // thing, and a caller acts on it in a third way.
        assert_eq!(
            identify(TWO, b"ZZ-DATA", None),
            Identification::Unrecognised
        );
        assert_eq!(identify(TWO, b"", None), Identification::Unrecognised);
    }

    #[test]
    fn an_ambiguous_file_names_every_reader_that_claimed_it() {
        // The message that turns a report into a fix. Naming one of the two
        // would leave the person reading it unable to say which rule is wrong.
        let ambiguous = identify(OVERLAPPING, b"ALPHA-DATA", None);
        let Identification::Ambiguous(claimants) = &ambiguous else {
            panic!("two overlapping predicates did not produce an ambiguity: {ambiguous:?}");
        };
        let named: Vec<&str> = claimants.iter().map(|info| info.id.as_str()).collect();
        assert_eq!(named, vec!["alpha", "loose"]);

        let sentence = ambiguous.to_string();
        assert!(
            sentence.contains("alpha") && sentence.contains("loose"),
            "{sentence}"
        );
        assert!(
            sentence.contains("defect in this software"),
            "the ambiguity blames the file: {sentence}"
        );
    }

    #[test]
    fn the_extension_orders_the_answer_and_never_decides_it() {
        // The bytes are beta's and the name says alpha's. The bytes win, and
        // this is the whole of what the rule protects: a file somebody renamed
        // is still read by what is in it.
        let by_bytes = identify(TWO, b"BETA-DATA", Some("sweep.alp"));
        match by_bytes {
            Identification::Recognised(info) => assert_eq!(info.id, "beta"),
            other => panic!("the extension changed the answer: {other:?}"),
        }

        // A name matching nothing in the file adds no claimant.
        assert_eq!(
            identify(TWO, b"ZZ-DATA", Some("sweep.alp")),
            Identification::Unrecognised
        );

        // What it does do: among readers that all claimed the file, the one
        // whose extension matches is named first, so a person reading the list
        // starts with the likely one. The set is unchanged.
        let Identification::Ambiguous(unordered) = identify(OVERLAPPING, b"ALPHA", None) else {
            panic!("the fixture is not ambiguous");
        };
        let Identification::Ambiguous(ordered) = identify(OVERLAPPING, b"ALPHA", Some("x.dat"))
        else {
            panic!("the extension removed a claimant");
        };
        let before: Vec<&str> = unordered.iter().map(|info| info.id.as_str()).collect();
        let after: Vec<&str> = ordered.iter().map(|info| info.id.as_str()).collect();
        assert_eq!(before, vec!["alpha", "loose"]);
        assert_eq!(after, vec!["loose", "alpha"], "the order did not move");

        let mut before_sorted = before.clone();
        let mut after_sorted = after.clone();
        before_sorted.sort_unstable();
        after_sorted.sort_unstable();
        assert_eq!(
            before_sorted, after_sorted,
            "the extension changed which readers claimed the file"
        );
    }

    #[test]
    fn a_name_with_no_extension_is_not_an_extension() {
        // Instrument files routinely have none, and a name with a dot in the
        // directory rather than the file is the case that gets this wrong.
        assert_eq!(extension_of("sweep"), None);
        assert_eq!(extension_of(".hidden"), None);
        assert_eq!(extension_of("sweep."), None);
        assert_eq!(extension_of("sweep.raw.DAT").as_deref(), Some("dat"));
        assert_eq!(extension_of("/some.dir/sweep").as_deref(), None);
        assert_eq!(
            extension_of("C:\\some.dir\\sweep.alp").as_deref(),
            Some("alp")
        );
    }

    #[test]
    fn the_prefix_is_bounded_and_the_source_comes_back_wound_to_the_start() {
        // Twice the bound, so that a prefix reader that read the whole file
        // would be visible as a length rather than as a performance problem
        // nobody measures.
        let long = vec![0x41_u8; RECOGNITION_PREFIX * 2];
        let mut source = std::io::Cursor::new(long);
        let Ok(prefix) = prefix_of(&mut source) else {
            panic!("a source in memory could not be read");
        };
        assert_eq!(prefix.len(), RECOGNITION_PREFIX);
        assert_eq!(source.position(), 0, "the source was not wound back");

        // A file shorter than the bound is shown what there is. The bound is a
        // maximum; requiring it would decline every short file.
        let mut short = std::io::Cursor::new(b"AL".to_vec());
        assert_eq!(prefix_of(&mut short).ok().map(|got| got.len()), Some(2));

        // And a caller who wound the source forward still gets the start of the
        // file, because identification is about the front of it.
        let mut wound = std::io::Cursor::new(b"ALPHA".to_vec());
        wound.set_position(3);
        assert_eq!(
            prefix_of(&mut wound).ok().as_deref(),
            Some(b"ALPHA".as_slice())
        );
    }

    #[test]
    fn a_slice_longer_than_the_bound_is_cut_before_any_predicate_sees_it() {
        // The bound holds against the caller and not only against the source. A
        // predicate is promised at most this many bytes and the fixture reader
        // above asserts that promise, so a cut that stopped happening reaches
        // this test as a failure rather than as a slower run nobody measures.
        let long = vec![0x41_u8; RECOGNITION_PREFIX * 2];
        assert_eq!(identify(TWO, &long, None), Identification::Unrecognised);

        // And the claim still lands on the bytes inside the bound, so the cut
        // is not doing its work by refusing everything long.
        let mut claimed = vec![0x5a_u8; RECOGNITION_PREFIX * 2];
        claimed.splice(..2, b"AL".iter().copied());
        match identify(TWO, &claimed, None) {
            Identification::Recognised(info) => assert_eq!(info.id, "alpha"),
            other => panic!("a long file with a magic at its front was not claimed: {other:?}"),
        }
    }
}
