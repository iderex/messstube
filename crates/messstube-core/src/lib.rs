//! The library is the product. This crate holds the measurement types, the
//! reader interface and the identification registry.
//!
//! The measurement types are here, from #31: [`measurement`] holds what a read
//! produces and [`unit`] holds the vocabulary a channel or an axis states its
//! quantity in. Neither carries reader logic or format knowledge, and neither
//! reads or writes anything.
//!
//! What a read says when it does not produce one of those is [`error`], from
//! #34: the two failure kinds, the absolute byte offset a damaged file carries,
//! and the option that turns a refusal into a partial result with its losses
//! named.
//!
//! [`reader`] is the interface every reader implements and the registry that
//! holds them, from #32. No reader is compiled in yet, so the registry there is
//! empty; the first reader is #48.
//!
//! [`bounded`] is what reader code reads through, from #35: the cursor that
//! cannot leave the bytes it was given, the allocation that is checked against
//! the file before anything is reserved, the depth guard and the bounded string
//! reader. It is the hostile-input budget in
//! `docs/decisions/0007-hostile-input-budget.md` as one implementation rather
//! than as a rule every reader author has to remember.
//!
//! [`read`] is the path a caller uses instead of calling a reader directly, and
//! [`provenance`] is what that path attaches to every measurement it produces,
//! both from #36. The content hash in the block is [`hash`], written out here
//! rather than taken from a crate for the reason that module gives.
//!
//! [`write`] is the other end: the delimited sample table and the metadata
//! document 0008 fixes as what the core writes, from #38. Both are plain text,
//! both take no dependency, and both produce the same bytes on every machine.
//!
//! [`identify`] is what chooses the reader [`read::read_with`] is told to use,
//! from #33. It runs every recognition predicate over one bounded prefix and
//! answers one of three ways: exactly one reader claims the file, several do
//! and it names all of them, or none does. The last two are refusals and a
//! caller acts on them differently.
//!
//! The shape of this crate's public interface is constrained by
//! `docs/decisions/0002-product-surface.md`: plain owned data rather than
//! borrowed views, no lifetimes in signatures, and error values describable
//! without Rust vocabulary. Those hold from the first reader, not from the day
//! a language binding is attempted.

// Part two of the hostile-input budget in
// docs/decisions/0007-hostile-input-budget.md. At the crate root rather than in
// review, and `forbid` rather than `deny` so it cannot be relaxed further down
// by an attribute somebody adds in a hurry.
#![forbid(unsafe_code)]

pub mod bounded;
pub mod error;
pub mod hash;
pub mod identify;
pub mod measurement;
pub mod provenance;
pub mod read;
pub mod reader;
pub mod unit;
pub mod write;

#[cfg(test)]
mod tests {
    //! The worked example of a unit test, from `docs/testing.md`. A unit test
    //! lives beside the thing it tests, which is why this block is in the file
    //! it is about rather than in `tests/`.
    //!
    //! What it is about is the fixture rule itself, because that rule is the
    //! one part of the test conventions that has something to assert before any
    //! reader exists.

    // Turned off for test code only: naming a byte by its offset in a fixture of
    // known length is the assertion, and rewriting it through a fallible lookup
    // would hide what the test is checking behind the checking.
    #![allow(clippy::indexing_slicing)]

    /// A truncated header, written the way `docs/testing.md` requires: an
    /// escaped byte-string literal in the source rather than a file in the
    /// tree. The two bytes that matter are at offsets 6 and 7.
    const TRUNCATED_HEADER: &[u8] = b"\x4d\x53\x54\x42\x00\x00\x0d\x0a";

    #[test]
    fn an_escaped_fixture_keeps_the_carriage_return_it_was_written_with() {
        // The point of the rule. Committed as a raw file, this pair would be
        // rewritten to a lone 0x0a by the checkout on at least one platform,
        // and the fixture would go on passing while testing something else.
        // There is nothing in `\x0d` for a checkout to normalise.
        assert_eq!(TRUNCATED_HEADER[6], 0x0d);
        assert_eq!(TRUNCATED_HEADER[7], 0x0a);
        assert_eq!(TRUNCATED_HEADER.len(), 8);
    }
}
