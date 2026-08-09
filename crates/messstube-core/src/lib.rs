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
//! Identification over a bounded prefix is #33. Its module layout is left to
//! that issue rather than guessed at here.
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

pub mod error;
pub mod measurement;
pub mod reader;
pub mod unit;

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
