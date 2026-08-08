//! The library is the product. This crate holds the measurement types, the
//! reader interface and the identification registry.
//!
//! None of those exist yet. This crate is the skeleton from #14 and carries no
//! reader logic and no format knowledge, so that the scaffolding could be
//! reviewed as scaffolding. The measurement types are #31, the reader interface
//! and the compile-time registry are #32, and identification over a bounded
//! prefix is #33. The module layout is left to those issues rather than guessed
//! at here.
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
