//! The worked example of an integration test, from `docs/testing.md`.
//!
//! An integration test is compiled as a separate crate that depends on this one,
//! so it can only reach the public interface. That is the property that makes
//! the kind worth having: it fails when something a caller depends on moves, and
//! it does not fail when an internal detail is rearranged.
//!
//! The public interface holds the measurement types, from #31. #32 adds the
//! reader interface and the registry, #33 identification, and this file is
//! still the shape those issues write into.

// The library's own crate root forbids unsafe code. A test target is a separate
// crate and does not inherit that, so it says so itself.
#![forbid(unsafe_code)]
// Turned off for this test target only, the same way the unit test modules in
// the library do it: a test whose precondition does not hold has to stop
// loudly, and `expect` with a sentence in it says which precondition that was.
// What the workspace lint set denies these for is library code, which may not
// end the process of the program that linked it.
#![allow(clippy::expect_used)]

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

/// A caller builds a measurement out of the public types and reads a physical
/// value back out of it. That is the whole of what a reader has to be able to
/// produce and what a writer has to be able to consume, so it is the smallest
/// thing that fails when the type stops being usable from outside.
///
/// It also holds two of the three constraints in
/// `docs/decisions/0002-product-surface.md` where they can actually be seen:
/// nothing below names a lifetime, and everything it gets back is owned. The
/// third, that error values are describable without Rust vocabulary, has a
/// subject since #34 landed the error type, and it is asserted where that type
/// is exercised, in `crates/messstube-core/tests/error_model.rs`.
#[test]
fn a_caller_can_build_a_measurement_and_read_a_physical_value_from_it() {
    use messstube_core::measurement::{
        Axis, AxisShape, Channel, Measurement, Samples, Transform, Uncertainty,
    };
    use messstube_core::unit::Unit;

    let measurement = Measurement::new(
        vec![Channel {
            name: "Ch2".to_owned(),
            unit: Unit::Volt,
            // Two sixteen-bit codes, unscaled and unshifted as the file held
            // them.
            samples: Samples::I16(vec![0, 16_384]),
            transform: Transform {
                scale: 1.0 / 32_768.0,
                offset: 0.0,
            },
            uncertainty: Some(Uncertainty::Absolute(0.001)),
        }],
        vec![Axis {
            name: "time".to_owned(),
            unit: Unit::Second,
            shape: AxisShape::Regular {
                start: 0.0,
                step: 400e-9,
                count: 2,
            },
        }],
    );

    let channel = measurement
        .channels
        .first()
        .expect("the measurement was built with one channel");

    // The code is what the instrument stored, at the width it stored it.
    assert_eq!(channel.samples.stored_bits(), 16);

    // The physical value is arithmetic this test does itself, so what is
    // asserted is that the transform was applied on request rather than a
    // number obtained from anywhere else. There is no independently obtained
    // value here and none is claimed; `docs/testing.md` is where that
    // distinction is fixed.
    let physical = channel
        .physical(1)
        .expect("index 1 is inside a two-sample channel");
    assert!(
        (physical - 0.5).abs() < 1e-12,
        "16384 of 32768 volts full scale is 0.5, and this gave {physical}"
    );

    let axis = measurement
        .axes
        .first()
        .expect("the measurement was built with one axis");
    let position = axis
        .shape
        .position(1)
        .expect("index 1 is inside a two-point axis");
    assert!((position - 400e-9).abs() < 1e-18, "{position}");

    // A stated uncertainty comes back as stated, and nothing converted it.
    assert_eq!(channel.uncertainty, Some(Uncertainty::Absolute(0.001)));
}
