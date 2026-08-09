//! The interface every reader implements and the registry that holds them,
//! from #32.
//!
//! SMALL ON PURPOSE. An interface a format cannot fit is an interface that gets
//! bypassed, and one bypass is the end of every guarantee the rest of this crate
//! makes. So a reader declares six things and does one, and anything a format
//! needs beyond that is an argument for changing this file rather than for going
//! around it.
//!
//! THE READ TAKES A SEEKABLE SOURCE AND NOT A PATH. A reader that opens its own
//! input cannot be tested from an in-memory fixture, which is what
//! `docs/decisions/0011-headless-testing.md` requires of the default suite, and
//! it has also read something the caller did not hand it, which is part four of
//! `docs/decisions/0007-hostile-input-budget.md`. [`Source`] is satisfied by
//! `std::io::Cursor` over a byte slice, so every reader is testable with no
//! filesystem at all.
//!
//! THE READ TAKES AN OPTIONS VALUE AND NOT A LIST OF ARGUMENTS. The option that
//! exists today is the opt-in partial read from #34. The next one is a field on
//! [`ReadOptions`](crate::error::ReadOptions) rather than a signature change to
//! every reader in the tree by then.
//!
//! THE REGISTRY IS BUILT AT COMPILE TIME. No dynamic loading and no plugin
//! mechanism: a shared object loaded into this process is memory-unsafe input by
//! another name, and it would defeat the whole language argument in
//! `docs/decisions/0002-product-surface.md`. A [`Registry`] is a slice of
//! readers fixed when the binary was linked, and the only way to add one is to
//! compile it in.
//!
//! IT IS ENUMERABLE BECAUSE NOTHING MAY BE MAINTAINED BY HAND. The tool has to
//! print what it supports and the documentation table has to come from the code,
//! and a supported-formats list a person maintains is a list that is wrong. The
//! enumeration hands back plain owned data, so the answer is describable to a
//! caller that is not written in Rust.
//!
//! WHAT IS NOT HERE. [`Registry`] holds no readers in this tree yet, because the
//! first one is #48 and every reader crate is its own crate under
//! `crates/readers/`. So [`COMPILED_IN`] is empty, and it stays empty after #48
//! as well: a reader crate depends on this crate, so the registry a build
//! actually ships is assembled by whichever crate links the readers together,
//! which is the tool in #37. The tests below are accordingly over registries
//! built for the test, which is also what makes them a proof of the guard rather
//! than a report on the state of this tree on the day they ran.

use crate::error::{ReadError, ReadOptions, ReadOutcome};
use core::fmt;
use core::fmt::Write as _;
use std::io::{Read, Seek};

/// The most bytes a recognition predicate may be shown.
///
/// The bound is here rather than per reader, so that identification can read one
/// prefix once and offer it to every reader, and so that no reader can make
/// recognition cost a whole file. A predicate that cannot decide inside this
/// many bytes is a predicate that has to say no.
///
/// The number is the smallest that comfortably covers a header: the four
/// families in `docs/landscape.md` all state their magic, their version and
/// their record counts well inside it. Raising it is a change with a reason
/// recorded, and never a thing a reader does for itself.
pub const RECOGNITION_PREFIX: usize = 512;

/// The instrument family a reader belongs to.
///
/// The four surveyed in `docs/landscape.md`, and it is a closed set for the same
/// reason `AxisShape` is: a fifth family is an argument somebody has in an issue,
/// with a survey behind it, rather than a variant that appears inside a reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    /// Oscilloscope raw formats.
    Oscilloscope,
    /// Profilometers and surface metrology.
    SurfaceMetrology,
    /// Hall measurement rigs.
    Hall,
    /// Vacuum and sputter process controllers.
    ProcessController,
}

impl fmt::Display for Family {
    /// The words `docs/landscape.md` uses, so a table generated from the code
    /// and the page a person reads name the same thing.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let written = match self {
            Family::Oscilloscope => "oscilloscope",
            Family::SurfaceMetrology => "surface metrology",
            Family::Hall => "Hall measurement",
            Family::ProcessController => "process controller",
        };
        formatter.write_str(written)
    }
}

/// What evidence stands behind a reader, from
/// `docs/decisions/0009-reader-maturity.md`.
///
/// Cut by evidence and by nothing else. It says nothing about how complete the
/// format coverage is, which is a different axis and belongs in the format note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Maturity {
    /// A reader exists and has been verified against no real file. Nothing may
    /// depend on it.
    Sketched,
    /// Verified against real files from at least one physical instrument. The
    /// parse is known to work on bytes a machine produced; the numbers are not
    /// claimed.
    Verified,
    /// Everything `Verified` claims, and the values agree with values obtained
    /// independently of this repository.
    Corroborated,
    /// Superseded or withdrawn, with the reason recorded. A reader does not
    /// silently disappear, because somebody has output that came from it.
    Retired,
}

impl fmt::Display for Maturity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let written = match self {
            Maturity::Sketched => "sketched",
            Maturity::Verified => "verified",
            Maturity::Corroborated => "corroborated",
            Maturity::Retired => "retired",
        };
        formatter.write_str(written)
    }
}

/// A byte source a reader can move around in.
///
/// It is the caller's handle on the input and never something the reader opened.
/// Anything that reads and seeks satisfies it, which includes `std::io::Cursor`
/// over bytes already in memory, so a reader is testable with no filesystem.
///
/// It is a trait of its own rather than two bounds written out at every call
/// site, so that the shape a reader takes is one name that can be found.
pub trait Source: Read + Seek {}

impl<T: Read + Seek + ?Sized> Source for T {}

/// What every reader implements.
///
/// Six declarations and one action. The declarations are what the registry, the
/// tool and the generated table read; the action is the only thing that touches
/// bytes.
///
/// `Sync` is required so a registry can be a shared constant in a program that
/// uses more than one thread. Every reader in this design is a unit value with
/// no state, so it costs nothing to satisfy and it is cheaper to require now
/// than to retrofit.
pub trait Reader: Sync {
    /// The stable identifier. It appears in errors, in the provenance block and
    /// in the tool's output, so it is chosen once and does not change: something
    /// somebody wrote down two years ago has to keep meaning this reader.
    fn id(&self) -> String;

    /// What to call it in front of a person.
    fn name(&self) -> String;

    /// The instrument family it belongs to.
    fn family(&self) -> Family;

    /// What evidence stands behind it today. A reader declares this in code and
    /// the verification ledger is what checks the declaration against the
    /// corpus.
    fn maturity(&self) -> Maturity;

    /// The file extensions this format is usually saved under, lower-cased and
    /// without their dots.
    ///
    /// A HINT AND NEVER AN ANSWER. Identification decides on the bytes; this
    /// only puts the likely reader at the front of a list a person reads, in
    /// [`identify`](crate::identify::identify). A reader declaring an extension
    /// gains no claim on a file that carries it, and a reader declaring none
    /// loses nothing, which is why the default is none: an instrument file
    /// often has no extension at all, and several families share `.dat`.
    ///
    /// Defaulted rather than required, from #33, so that a reader with nothing
    /// useful to say here says nothing rather than inventing a suffix.
    fn extensions(&self) -> Vec<String> {
        Vec::new()
    }

    /// Whether this reader claims the file, judged on at most
    /// [`RECOGNITION_PREFIX`] bytes from its start.
    ///
    /// The caller supplies what it has, which may be shorter than the bound
    /// where the file is shorter. A reader that cannot decide on what it was
    /// given answers `false`: claiming a file it cannot recognise turns another
    /// reader's damaged file into this one's, which is the confusion the two
    /// error kinds in #34 exist to prevent.
    fn recognises(&self, prefix: &[u8]) -> bool;

    /// Turn bytes into a measurement, or say why not.
    ///
    /// The source is positioned by the reader itself; nothing may be assumed
    /// about where the caller left it. Errors carry an absolute offset into the
    /// input, which is the offset in this source and not in whatever the caller
    /// sliced it out of.
    ///
    /// # Errors
    ///
    /// [`ReadError::NotThisFormat`] where the file turned out not to be this
    /// reader's after all, and [`ReadError::Damaged`] where it was and something
    /// inside it is wrong.
    fn read(&self, source: &mut dyn Source, options: ReadOptions)
    -> Result<ReadOutcome, ReadError>;
}

/// What the registry says about one reader, as plain owned data.
///
/// This is the enumeration a caller sees. It is owned and names no lifetime, so
/// the answer survives being handed across a language binding, which is
/// constraints 1 and 2 of `docs/decisions/0002-product-surface.md`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReaderInfo {
    /// The reader's stable identifier.
    pub id: String,
    /// What to call it in front of a person.
    pub name: String,
    /// The instrument family it belongs to.
    pub family: Family,
    /// What evidence stands behind it.
    pub maturity: Maturity,
    /// The extensions it declares, which order an answer and never decide one.
    pub extensions: Vec<String>,
}

/// The readers compiled into this build.
///
/// A slice fixed at link time. `'static` appears here and nowhere a caller has
/// to carry it: [`Registry::describe`] is what a caller enumerates, and that
/// hands back owned values.
#[derive(Clone, Copy)]
pub struct Registry {
    entries: &'static [&'static dyn Reader],
}

impl fmt::Debug for Registry {
    /// The identifiers rather than the readers, because a reader is a unit value
    /// whose debug output says nothing, and what somebody debugging a registry
    /// wants to know is which readers are in it.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_list()
            .entries(self.entries.iter().map(|reader| reader.id()))
            .finish()
    }
}

impl Registry {
    /// Build a registry over readers compiled into this binary.
    ///
    /// The entries are supplied by whichever crate links the reader crates,
    /// because a reader crate depends on this one and the dependency cannot run
    /// both ways. This crate defines what a registry is; it does not decide who
    /// is in one.
    #[must_use]
    pub const fn new(entries: &'static [&'static dyn Reader]) -> Self {
        Registry { entries }
    }

    /// How many readers are compiled in.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no reader is compiled in.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every reader, described, in the order they were compiled in.
    ///
    /// The order is the declaration order and not a sort, so that a table
    /// generated from this is stable between runs and between machines.
    #[must_use]
    pub fn describe(&self) -> Vec<ReaderInfo> {
        self.entries
            .iter()
            .map(|reader| ReaderInfo {
                id: reader.id(),
                name: reader.name(),
                family: reader.family(),
                maturity: reader.maturity(),
                extensions: reader.extensions(),
            })
            .collect()
    }

    /// Every reader in this registry that claims these bytes.
    ///
    /// EVERY predicate is asked, and there is no early exit. An ambiguity is
    /// only visible if the second claimant was asked, and taking the first
    /// match would make the answer depend on link order for exactly the files
    /// where two rules overlap. What to do with more than one is
    /// [`identify`](crate::identify::identify)'s decision, not this one's.
    ///
    /// The prefix is the caller's, bounded by
    /// [`RECOGNITION_PREFIX`] on the way in.
    #[must_use]
    pub fn claimants(&self, prefix: &[u8]) -> Vec<ReaderInfo> {
        let shown = prefix.get(..RECOGNITION_PREFIX).unwrap_or(prefix);
        self.entries
            .iter()
            .filter(|reader| reader.recognises(shown))
            .map(|reader| ReaderInfo {
                id: reader.id(),
                name: reader.name(),
                family: reader.family(),
                maturity: reader.maturity(),
                extensions: reader.extensions(),
            })
            .collect()
    }

    /// The identifiers more than one reader in this registry declares.
    ///
    /// Empty where every identifier is unique, which is the state the suite
    /// requires of any registry this repository ships. Two readers under one
    /// identifier make the answer to "which reader produced this output" depend
    /// on link order, and that answer is written into the provenance block of
    /// every file somebody keeps.
    #[must_use]
    pub fn duplicate_identifiers(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        let mut duplicated: Vec<String> = Vec::new();
        for reader in self.entries {
            let id = reader.id();
            if seen.contains(&id) {
                if !duplicated.contains(&id) {
                    duplicated.push(id);
                }
            } else {
                seen.push(id);
            }
        }
        duplicated
    }

    /// The supported-formats table, generated from the enumeration.
    ///
    /// Markdown, because that is what this repository's documentation is
    /// written in, and generated rather than typed because a list a person
    /// maintains is a list that is wrong. Where nothing is compiled in it says
    /// so in a sentence rather than emitting an empty table, since an empty
    /// table reads like a formatting accident and a sentence does not.
    #[must_use]
    pub fn documentation_table(&self) -> String {
        let described = self.describe();
        if described.is_empty() {
            return "No reader is compiled into this build.\n".to_owned();
        }

        let mut table = String::from("| Identifier | Reader | Family | Maturity |\n");
        table.push_str("| --- | --- | --- | --- |\n");
        for info in described {
            let ReaderInfo {
                id,
                name,
                family,
                maturity,
                // The table names what a reader is and how far it is trusted.
                // An extension is a hint identification uses and not something
                // a person choosing a reader acts on.
                extensions: _,
            } = info;
            // Writing into a `String` has no error path, so there is no result
            // worth carrying: `write_str` under it returns `Ok` unconditionally.
            let _ = writeln!(table, "| `{id}` | {name} | {family} | {maturity} |");
        }
        table
    }
}

/// The readers compiled into this crate on its own.
///
/// Empty, and that is the state of the tree rather than a placeholder that will
/// be filled in here. Reader crates depend on this crate, so the list of them
/// belongs to whichever crate links them together, and this constant is what a
/// caller with no readers linked gets: an enumeration that honestly says
/// nothing is supported.
pub const COMPILED_IN: Registry = Registry::new(&[]);

#[cfg(test)]
mod tests {
    //! The registry's own properties, proved over fixture registries.
    //!
    //! FIXTURE REGISTRIES, NOT THIS TREE'S. A test judging [`COMPILED_IN`]
    //! would prove the state of the tree on the day it ran rather than the
    //! guard, and the state of the tree is that no reader exists yet. What is
    //! proved here is what the registry does with the readers it is given.

    use super::{
        COMPILED_IN, Family, Maturity, RECOGNITION_PREFIX, Reader, ReaderInfo, Registry, Source,
    };
    use crate::error::{ReadError, ReadOptions, ReadOutcome};
    use crate::measurement::Measurement;

    /// A reader that declines everything, so the tests below are about the
    /// registry rather than about parsing.
    struct Declining {
        id: &'static str,
    }

    impl Reader for Declining {
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
        fn recognises(&self, prefix: &[u8]) -> bool {
            // The bound is the interface's, and a reader may be shown less. A
            // predicate asserting it was shown the whole bound would refuse
            // every file shorter than it.
            assert!(prefix.len() <= RECOGNITION_PREFIX);
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

    const FIRST: Declining = Declining { id: "first" };
    const SECOND: Declining = Declining { id: "second" };
    const ALSO_FIRST: Declining = Declining { id: "first" };

    const TWO: Registry = Registry::new(&[&FIRST, &SECOND]);
    const COLLIDING: Registry = Registry::new(&[&FIRST, &ALSO_FIRST]);

    #[test]
    fn an_enumeration_keeps_the_order_the_readers_were_compiled_in() {
        // Declaration order rather than a sort, so a generated table is the same
        // on every machine and a difference in it means a difference in the
        // readers.
        let described = TWO.describe();
        assert_eq!(described.len(), 2);
        assert_eq!(TWO.len(), 2);
        assert!(!TWO.is_empty());

        let ids: Vec<String> = described.iter().map(|info| info.id.clone()).collect();
        assert_eq!(ids, vec!["first".to_owned(), "second".to_owned()]);

        assert_eq!(
            described.first(),
            Some(&ReaderInfo {
                id: "first".to_owned(),
                name: "the first fixture".to_owned(),
                family: Family::Oscilloscope,
                maturity: Maturity::Sketched,
                extensions: Vec::new(),
            })
        );
    }

    #[test]
    fn two_readers_under_one_identifier_are_reported_rather_than_preferred() {
        // The property that outlives an empty registry. Link order deciding
        // which reader answers to an identifier would put a different answer in
        // the provenance block of two otherwise identical runs.
        assert_eq!(TWO.duplicate_identifiers(), Vec::<String>::new());
        assert_eq!(
            COLLIDING.duplicate_identifiers(),
            vec!["first".to_owned()],
            "a colliding identifier was not reported"
        );
    }

    #[test]
    fn the_documentation_table_comes_out_of_the_enumeration() {
        // One row per reader and no hand-written list anywhere. What is checked
        // is that the row carries what the reader declared, because a table
        // generated from something other than the code is a hand-maintained
        // table with extra steps.
        let table = TWO.documentation_table();
        let rows = table.lines().count();
        assert_eq!(rows, 4, "a header, a rule and one row per reader: {table}");
        assert!(table.contains("| `first` | the first fixture | oscilloscope | sketched |"));
        assert!(table.contains("| `second` | the second fixture | oscilloscope | sketched |"));
    }

    #[test]
    fn an_empty_registry_says_so_in_a_sentence() {
        // The state of this tree today, and the state a build with no reader
        // crates linked will always be in. An empty markdown table reads as a
        // formatting accident.
        assert!(COMPILED_IN.is_empty());
        assert_eq!(COMPILED_IN.len(), 0);
        assert_eq!(COMPILED_IN.describe(), Vec::new());
        assert_eq!(
            COMPILED_IN.documentation_table(),
            "No reader is compiled into this build.\n"
        );
    }

    #[test]
    fn a_reader_reads_from_bytes_in_memory_with_no_filesystem() {
        // What the seekable source buys, and the reason the interface does not
        // take a path: every reader is testable from a fixture, which is what
        // the default suite is required to be able to do.
        let mut source = std::io::Cursor::new(b"MSTB".to_vec());
        let refused = FIRST.read(&mut source, ReadOptions::default());
        assert_eq!(
            refused,
            Err(ReadError::NotThisFormat {
                reader: "first".to_owned()
            })
        );

        // And the recognition predicate is offered a prefix rather than the
        // file. A file shorter than the bound is shown what there is.
        assert!(!FIRST.recognises(b"MSTB"));
        assert!(!FIRST.recognises(&[0_u8; RECOGNITION_PREFIX]));
    }

    #[test]
    fn the_read_returns_what_a_read_produces_and_nothing_of_its_own() {
        // The interface is the join between #31 and #34: a measurement or one of
        // the two error kinds, and no third shape invented at this seam.
        fn accepts(_: Result<ReadOutcome, ReadError>) {}
        let empty = Measurement::new(Vec::new(), Vec::new());
        accepts(Ok(ReadOutcome::complete(empty)));
    }
}
