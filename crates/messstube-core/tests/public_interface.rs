//! The worked example of an integration test, from `docs/testing.md`.
//!
//! An integration test is compiled as a separate crate that depends on this one,
//! so it can only reach the public interface. That is the property that makes
//! the kind worth having: it fails when something a caller depends on moves, and
//! it does not fail when an internal detail is rearranged.
//!
//! The public interface is empty today. #31 adds the measurement types, #32 the
//! reader interface and the registry, #33 identification. Until then this file
//! is the shape those issues write into rather than a test of anything.

// The library's own crate root forbids unsafe code. A test target is a separate
// crate and does not inherit that, so it says so itself.
#![forbid(unsafe_code)]

/// This asserts nothing at run time, and that is stated rather than dressed up.
/// What it does prove is a compile-time fact: `messstube_core` is nameable and
/// linkable from outside itself, which is what every later test in this file
/// depends on and which would otherwise be discovered by the first person to
/// write a real one.
#[test]
fn the_library_is_reachable_from_outside_its_own_crate() {
    // A `use` of the crate root is the whole assertion; if the crate did not
    // link, this file would not compile.
    #[allow(unused_imports)]
    use messstube_core as _;
}
