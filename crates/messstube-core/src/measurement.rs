//! What a read produces, from #31, implementing
//! `docs/decisions/0004-what-a-read-produces.md`.
//!
//! One kind of value, for every reader in this repository. It is fixed before
//! the first reader because changing it afterwards changes every reader at once.
//!
//! THE STORED CODES AND THE TRANSFORM ARE KEPT APART. That is the load-bearing
//! part of 0004 and the reason this module has a [`Transform`] beside
//! [`Samples`] rather than a vector of physical values. Multiplying on the way
//! out rounds an exact stored value, so a round trip stops being one; it hides
//! that the instrument quantised at a particular step, which is what somebody
//! re-analysing an old measurement needs; and it hides digitiser saturation,
//! because a clipped code is recognisable against the width it was stored in and
//! a clipped floating point value is not.
//!
//! NOTHING HERE READS OR WRITES ANYTHING. Reading produces a measurement and
//! writing consumes one, both from elsewhere, so that this type does not
//! accumulate a dependency for every format anybody ever adds. That is a
//! property this file is checked against rather than only asked for, in
//! `no_type_in_this_set_has_an_input_or_output_method` at the bottom.
//!
//! WHAT THIS TYPE DOES NOT HOLD, recorded rather than discovered later. Two of
//! the four families in `docs/landscape.md` place requirements on it that it
//! does not meet, and that page records both against this issue and against the
//! interface review in #59.
//!
//! The Hall rigs need the contact geometry of each reading, the sign of the
//! field and of the current for each reading, and the thickness of the sample,
//! which is a property of the sample rather than of the measurement. A reader
//! returning those voltages as channels on an axis and dropping the permutation
//! labels would produce something that reads like a measurement and is not one.
//!
//! The process controllers need discrete states, events that happen at a time
//! rather than over one, and an answer to where one measurement starts and stops
//! inside a continuous log. Sampling a valve position onto a time grid to make
//! it look like a channel invents values between the events.
//!
//! Neither is answered here. Both are interface questions the surveys
//! deliberately left to #59, and inventing an answer inside the type work would
//! be taking a decision in the place where it is least visible.

use crate::provenance::{Instrument, Provenance};
use crate::unit::Unit;

/// What one read produced.
///
/// The channels are the measured quantities. The axes are what the samples in
/// every channel sit on, in order, so a channel of a two-dimensional scan has
/// two axes and its samples are laid out with the last axis varying fastest.
///
/// The provenance block 0004 requires is here and is not writable from outside
/// this crate. [`read_with`](crate::read::read_with) attaches it; a reader
/// supplies [`instrument`](Measurement::instrument) and nothing else of it. That
/// is #36's requirement that a reader can neither forget the block nor fill it
/// in wrongly, held by the type rather than by a review:
///
/// ```compile_fail
/// use messstube_core::measurement::Measurement;
///
/// let mut measurement = Measurement::new(Vec::new(), Vec::new());
/// measurement.provenance = None;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// One or more named channels. A model assuming exactly one is rejected by
    /// 0004, because every oscilloscope file breaks it.
    pub channels: Vec<Channel>,
    /// The axes the samples sit on, outermost first.
    pub axes: Vec<Axis>,
    /// What the file said about the instrument that wrote it, which is the one
    /// part of the provenance block a reader is the source of. `None` where the
    /// format carries no identification, and never a guess.
    pub instrument: Option<Instrument>,
    /// Where this came from. Private, and the read path is the only writer.
    ///
    /// `None` on a measurement that came straight out of a reader without going
    /// through the read path, which is a thing a caller can do and a thing this
    /// library never does. Reading through [`read_with`](crate::read::read_with)
    /// always produces `Some`, and
    /// `every_measurement_off_the_read_path_carries_a_block` in
    /// `crates/messstube-core/tests/provenance.rs` is where that is asserted.
    provenance: Option<Provenance>,
}

impl Measurement {
    /// What a reader hands back: channels, axes and nothing attached yet.
    ///
    /// A constructor rather than a struct literal, because the provenance field
    /// is private and a literal outside this crate could not name it. That is
    /// the mechanism, and this function is what keeps it from also being an
    /// obstacle to writing a reader.
    #[must_use]
    pub const fn new(channels: Vec<Channel>, axes: Vec<Axis>) -> Self {
        Measurement {
            channels,
            axes,
            instrument: None,
            provenance: None,
        }
    }

    /// Where this measurement came from, where it came through the read path.
    #[must_use]
    pub const fn provenance(&self) -> Option<&Provenance> {
        self.provenance.as_ref()
    }

    /// Attach the block. Inside this crate only, and called by the read path.
    pub(crate) fn attach(&mut self, provenance: Provenance) {
        self.provenance = Some(provenance);
    }
}

/// One named channel of samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    /// What the instrument called it. `Ch2` off an oscilloscope preamble, a
    /// gas flow's tag on a process controller.
    pub name: String,
    /// The unit of the PHYSICAL value, which is what the transform produces.
    /// The stored codes have no unit.
    pub unit: Unit,
    /// The codes as the instrument stored them, at the width it stored them.
    pub samples: Samples,
    /// How a stored code becomes a physical value. The identity where the
    /// instrument wrote physical values directly, which 0004 accepts as the cost
    /// of one shape across all families.
    pub transform: Transform,
    /// The uncertainty the instrument itself stated. `None` where it stated
    /// none, and nothing in this repository fills it in: a reader that invents
    /// an uncertainty is worse than one reporting none, because a downstream
    /// tool cannot tell the two apart.
    pub uncertainty: Option<Uncertainty>,
}

impl Channel {
    /// The physical value at an index, computed on request.
    ///
    /// `None` where the index is past the end. Out of range is not a panic here:
    /// this is library code handed data that came out of somebody else's file,
    /// and a panic in it ends the process of the program that linked the
    /// library.
    #[must_use]
    pub fn physical(&self, index: usize) -> Option<f64> {
        self.samples
            .code(index)
            .map(|code| self.transform.apply(code))
    }
}

/// One axis the samples sit on.
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    /// What the instrument called it, or what it is: `time`, `position`.
    pub name: String,
    /// The unit of the positions on this axis. A categorical axis carries
    /// [`Unit::Dimensionless`], because its positions are labels and a label has
    /// no unit.
    pub unit: Unit,
    /// Which of the three shapes this axis has.
    pub shape: AxisShape,
}

/// The three shapes an axis can have.
///
/// A small closed set, and it is closed on purpose. These three cover every
/// family in scope, and adding a fourth is a change somebody argues for in an
/// issue rather than a variant that appears inside a reader. The set is
/// exhaustive rather than open for the same reason: a fourth variant has to
/// break every caller that matches on this, which is what makes the argument
/// happen.
#[derive(Debug, Clone, PartialEq)]
pub enum AxisShape {
    /// A start and a step, repeated a fixed number of times. The oscilloscope
    /// case, where the preamble states an increment and a point count rather
    /// than a position per sample.
    Regular {
        /// The position of the first sample.
        start: f64,
        /// The distance from each sample to the next. Not required to be
        /// positive: an instrument sweeping downwards states a negative step,
        /// and rewriting it would be the reader deciding something the file
        /// already said.
        step: f64,
        /// How many positions there are.
        count: usize,
    },
    /// A position per sample, written out. The case where the interval is
    /// irregular, which 0004 refuses to assume away.
    Explicit(Vec<f64>),
    /// A labelled categorical axis, for a swept parameter that is not a number:
    /// a contact permutation, a recipe step, a named setting.
    Categorical(Vec<String>),
}

impl AxisShape {
    /// How many positions this axis has.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            AxisShape::Regular { count, .. } => *count,
            AxisShape::Explicit(positions) => positions.len(),
            AxisShape::Categorical(labels) => labels.len(),
        }
    }

    /// Whether the axis has no positions at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The numeric position at an index, or `None` where the axis has no
    /// numeric position there.
    ///
    /// A categorical axis always answers `None`, and that is the honest answer
    /// rather than an index cast to a number. A recipe step named `presputter`
    /// is not at position 3 on any scale; it is third in a list, and the two are
    /// different statements.
    #[must_use]
    pub fn position(&self, index: usize) -> Option<f64> {
        match self {
            AxisShape::Regular { start, step, count } => {
                if index < *count {
                    // `u32` rather than `usize` on the way to `f64`, because a
                    // `usize` to `f64` conversion loses precision above 2^53 and
                    // the lint set in this repository denies a lossy cast. An
                    // axis with more than four thousand million positions is not
                    // a thing any file in scope holds, and where one arrives
                    // this returns `None` rather than a wrong number.
                    let offset = u32::try_from(index).ok().map(f64::from)?;
                    Some(step.mul_add(offset, *start))
                } else {
                    None
                }
            }
            AxisShape::Explicit(positions) => positions.get(index).copied(),
            AxisShape::Categorical(_) => None,
        }
    }

    /// The label at an index, or `None` where the axis is not a labelled one.
    #[must_use]
    pub fn label(&self, index: usize) -> Option<String> {
        match self {
            AxisShape::Categorical(labels) => labels.get(index).cloned(),
            AxisShape::Regular { .. } | AxisShape::Explicit(_) => None,
        }
    }
}

/// The affine transform from a stored code to a physical value.
///
/// Kept beside the codes rather than applied to them. See the module
/// documentation for the three things applying it early destroys.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// What a code is multiplied by. This is also where a scale prefix goes: an
    /// instrument stating millivolts is a channel in volts whose scale carries
    /// the thousandth, rather than a `Millivolt` entry in the unit vocabulary.
    pub scale: f64,
    /// What is added afterwards. The vertical zero of an oscilloscope record
    /// sits here.
    pub offset: f64,
}

impl Transform {
    /// The transform for an instrument that wrote physical values directly.
    ///
    /// 0004 names this case and accepts that the structure then carries a field
    /// saying nothing, as the cost of one shape across all families rather than
    /// a shape per family.
    pub const IDENTITY: Transform = Transform {
        scale: 1.0,
        offset: 0.0,
    };

    /// Apply the transform to one code.
    #[must_use]
    pub fn apply(&self, code: f64) -> f64 {
        // `mul_add` rather than `self.scale * code + self.offset`. It rounds
        // once instead of twice, which is the difference between a value that
        // round-trips and one that is off by an ulp for a reason nobody can see
        // in the file.
        self.scale.mul_add(code, self.offset)
    }
}

/// The uncertainty the instrument stated.
///
/// Two shapes, because instruments state accuracy both ways: as a magnitude in
/// the channel's own unit, and as a fraction of the reading. Nothing here
/// converts one into the other, because doing so needs the reading and produces
/// a number the instrument did not state.
///
/// What this does not carry is a coverage factor or a distribution. No format
/// surveyed so far states one, and a field that every reader fills with a guess
/// is worse than an absent field for the reason 0004 gives about invented
/// uncertainty: it is indistinguishable from evidence.
#[derive(Debug, Clone, PartialEq)]
pub enum Uncertainty {
    /// Plus or minus this much, in the channel's own unit.
    Absolute(f64),
    /// Plus or minus this fraction of the reading. A stated one percent is
    /// `0.01` here and not `1.0`, so that no caller has to know which of the two
    /// conventions a reader used.
    Relative(f64),
}

/// The stored codes, at the width the instrument stored them.
///
/// A sixteen-bit digitiser stays sixteen bits, so that saturation stays visible:
/// a code at the end of its own range is recognisable, and the same value
/// widened into a float is not.
///
/// WHY THERE IS NO 64-BIT INTEGER VARIANT. Nothing in the four families surveyed
/// stores one, and a 64-bit code cannot be widened to `f64` without losing
/// precision, so [`code`](Samples::code) could not answer for it truthfully. The
/// day a format needs one, what it needs is that decision taken deliberately,
/// not a variant added here and a lossy cast added beside it.
#[derive(Debug, Clone, PartialEq)]
pub enum Samples {
    /// Signed 8-bit codes.
    I8(Vec<i8>),
    /// Signed 16-bit codes. The common digitiser width.
    I16(Vec<i16>),
    /// Signed 32-bit codes, which is also where a 24-bit converter's output
    /// sits.
    I32(Vec<i32>),
    /// Unsigned 8-bit codes.
    U8(Vec<u8>),
    /// Unsigned 16-bit codes.
    U16(Vec<u16>),
    /// Unsigned 32-bit codes.
    U32(Vec<u32>),
    /// Values the instrument wrote as 32-bit floating point. Not codes, so the
    /// transform beside them is normally the identity.
    F32(Vec<f32>),
    /// Values the instrument wrote as 64-bit floating point.
    F64(Vec<f64>),
}

impl Samples {
    /// How many samples there are.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Samples::I8(values) => values.len(),
            Samples::I16(values) => values.len(),
            Samples::I32(values) => values.len(),
            Samples::U8(values) => values.len(),
            Samples::U16(values) => values.len(),
            Samples::U32(values) => values.len(),
            Samples::F32(values) => values.len(),
            Samples::F64(values) => values.len(),
        }
    }

    /// Whether there are no samples at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The width of the stored representation, in bits.
    ///
    /// This is what makes "the instrument's own width" a thing a caller can read
    /// back rather than a property that only exists in the variant name.
    #[must_use]
    pub fn stored_bits(&self) -> u32 {
        match self {
            Samples::I8(_) | Samples::U8(_) => 8,
            Samples::I16(_) | Samples::U16(_) => 16,
            Samples::I32(_) | Samples::U32(_) | Samples::F32(_) => 32,
            Samples::F64(_) => 64,
        }
    }

    /// The stored code at an index, widened to `f64` and not yet transformed.
    ///
    /// Every variant above widens into `f64` exactly, which is why the variant
    /// list stops where it does. `None` where the index is past the end.
    #[must_use]
    pub fn code(&self, index: usize) -> Option<f64> {
        match self {
            Samples::I8(values) => values.get(index).copied().map(f64::from),
            Samples::I16(values) => values.get(index).copied().map(f64::from),
            Samples::I32(values) => values.get(index).copied().map(f64::from),
            Samples::U8(values) => values.get(index).copied().map(f64::from),
            Samples::U16(values) => values.get(index).copied().map(f64::from),
            Samples::U32(values) => values.get(index).copied().map(f64::from),
            Samples::F32(values) => values.get(index).copied().map(f64::from),
            Samples::F64(values) => values.get(index).copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    // Turned off for test code only, and for the same reason the gate verb's
    // tests turn it off: a test whose precondition does not hold has to stop
    // loudly, and `expect` with a sentence in it is the clearest way to say
    // which precondition that was. The library itself may not end the process
    // of the program that linked it, which is what the workspace lint set
    // denies these for.
    #![allow(clippy::expect_used)]

    use super::{Axis, AxisShape, Channel, Measurement, Samples, Transform, Uncertainty};
    use crate::unit::Unit;

    /// Approximate equality with a stated tolerance, because the assertions
    /// below are about arithmetic on floating point numbers and an exact
    /// comparison would be asserting a property of the representation rather
    /// than of the transform.
    fn close(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-12
    }

    #[test]
    fn a_regular_axis_gives_a_position_per_index_and_nothing_past_the_end() {
        // The oscilloscope shape, with the numbers taken from the ISF preamble
        // quoted in `docs/landscape.md`: a 400 ns increment starting at zero.
        // The preamble is real and the arithmetic below is this repository's
        // own, so what is asserted is the axis and not the format.
        let shape = AxisShape::Regular {
            start: 0.0,
            step: 400e-9,
            count: 10_000,
        };

        assert_eq!(shape.len(), 10_000);
        assert!(!shape.is_empty());

        let first = shape
            .position(0)
            .expect("index 0 is inside a 10000-point axis");
        let second = shape
            .position(1)
            .expect("index 1 is inside a 10000-point axis");
        let last = shape
            .position(9_999)
            .expect("index 9999 is the last of 10000");
        assert!(close(first, 0.0), "{first}");
        assert!(close(second, 400e-9), "{second}");
        assert!(close(last, 9_999.0 * 400e-9), "{last}");

        // One past the end is `None` rather than an extrapolation. A regular
        // axis can compute a position for any index, which is exactly why it has
        // to be told to stop.
        assert_eq!(shape.position(10_000), None);
        assert_eq!(shape.label(0), None);
    }

    #[test]
    fn a_regular_axis_may_step_downwards() {
        // A negative step is not an error to be normalised away. The file said
        // the sweep ran downwards, and rewriting it is the reader deciding
        // something the instrument already stated.
        let shape = AxisShape::Regular {
            start: 10.0,
            step: -2.5,
            count: 5,
        };

        let third = shape.position(2).expect("index 2 is inside a 5-point axis");
        assert!(close(third, 5.0), "{third}");
        assert_eq!(shape.position(5), None);
    }

    #[test]
    fn an_explicit_axis_answers_from_its_list_and_holds_an_irregular_interval() {
        // The case 0004 refuses to assume away: the interval between positions
        // is not constant, so no start-and-step can describe it.
        let shape = AxisShape::Explicit(vec![0.0, 0.5, 2.0, 8.0]);

        assert_eq!(shape.len(), 4);
        let third = shape.position(2).expect("index 2 is inside a 4-point axis");
        assert!(close(third, 2.0), "{third}");
        assert_eq!(shape.position(4), None);
        assert_eq!(shape.label(2), None);

        let empty = AxisShape::Explicit(Vec::new());
        assert!(empty.is_empty());
        assert_eq!(empty.position(0), None);
    }

    #[test]
    fn a_categorical_axis_answers_with_labels_and_never_with_a_number() {
        // The swept parameter that is not a number. The labels here are the
        // contact permutations `docs/landscape.md` records the Hall family as
        // needing, which is the case that shows why an index must not be
        // returned as a position: these four are not at 0, 1, 2 and 3 on any
        // scale.
        let shape = AxisShape::Categorical(vec![
            "A-B/C-D".to_owned(),
            "B-C/D-A".to_owned(),
            "C-D/A-B".to_owned(),
            "D-A/B-C".to_owned(),
        ]);

        assert_eq!(shape.len(), 4);
        assert_eq!(shape.label(1), Some("B-C/D-A".to_owned()));
        assert_eq!(shape.label(4), None);

        // The assertion this test exists for.
        assert_eq!(shape.position(0), None);
        assert_eq!(shape.position(1), None);
    }

    #[test]
    fn a_stored_code_keeps_its_width_and_is_transformed_only_on_request() {
        // A sixteen-bit digitiser at the positive end of its own range. The
        // point of keeping the code is that this is recognisably the largest
        // value the converter can produce, which is what saturation looks like.
        let channel = Channel {
            name: "Ch2".to_owned(),
            unit: Unit::Volt,
            samples: Samples::I16(vec![0, -32_768, 32_767]),
            transform: Transform {
                scale: 1.0 / 32_768.0,
                offset: 0.5,
            },
            uncertainty: None,
        };

        assert_eq!(channel.samples.stored_bits(), 16);
        assert_eq!(channel.samples.len(), 3);

        // The code is unchanged by being stored, and it is still the code.
        let clipped = channel
            .samples
            .code(2)
            .expect("index 2 is inside a 3-sample channel");
        assert!(close(clipped, 32_767.0), "{clipped}");

        // The physical value is computed when asked for, and not before.
        let physical = channel
            .physical(2)
            .expect("index 2 is inside a 3-sample channel");
        assert!(close(physical, 32_767.0 / 32_768.0 + 0.5), "{physical}");

        assert_eq!(channel.physical(3), None);
    }

    #[test]
    fn the_identity_transform_returns_the_code_it_was_given() {
        // The case 0004 names: an instrument that wrote physical values
        // directly, where the structure carries a field saying nothing.
        assert!(close(Transform::IDENTITY.apply(1.5), 1.5));
        assert!(close(Transform::IDENTITY.apply(-2.0), -2.0));
    }

    #[test]
    fn a_measurement_holds_several_channels_on_shared_axes() {
        // Two channels on one time axis, which is the shape 0004 rejects a
        // single-channel model for: every oscilloscope file breaks it.
        let axis = Axis {
            name: "time".to_owned(),
            unit: Unit::Second,
            shape: AxisShape::Regular {
                start: 0.0,
                step: 400e-9,
                count: 2,
            },
        };
        let measurement = Measurement::new(
            vec![
                Channel {
                    name: "Ch1".to_owned(),
                    unit: Unit::Volt,
                    samples: Samples::I16(vec![10, 20]),
                    transform: Transform::IDENTITY,
                    uncertainty: Some(Uncertainty::Relative(0.01)),
                },
                Channel {
                    name: "Ch2".to_owned(),
                    unit: Unit::Volt,
                    samples: Samples::I16(vec![30, 40]),
                    transform: Transform::IDENTITY,
                    uncertainty: None,
                },
            ],
            vec![axis],
        );

        assert_eq!(measurement.channels.len(), 2);
        assert_eq!(measurement.axes.len(), 1);

        // A stated uncertainty and an absent one are different values, which is
        // the whole property: nothing here fills the second one in.
        let stated = measurement
            .channels
            .first()
            .expect("the measurement was built with two channels");
        let unstated = measurement
            .channels
            .get(1)
            .expect("the measurement was built with two channels");
        assert_eq!(stated.uncertainty, Some(Uncertainty::Relative(0.01)));
        assert_eq!(unstated.uncertainty, None);
    }

    #[test]
    fn every_stored_width_widens_to_a_code_without_losing_the_value() {
        // The bound the variant list stops at. Each of these is the extreme of
        // its own width, and each one has to come back exactly, because the
        // whole argument for keeping codes is that they are exact.
        let cases: Vec<(Samples, f64)> = vec![
            (Samples::I8(vec![i8::MIN]), -128.0),
            (Samples::I16(vec![i16::MIN]), -32_768.0),
            (Samples::I32(vec![i32::MAX]), 2_147_483_647.0),
            (Samples::U8(vec![u8::MAX]), 255.0),
            (Samples::U16(vec![u16::MAX]), 65_535.0),
            (Samples::U32(vec![u32::MAX]), 4_294_967_295.0),
            (Samples::F32(vec![0.5]), 0.5),
            (Samples::F64(vec![0.1]), 0.1),
        ];

        for (samples, expected) in cases {
            let code = samples.code(0).expect("each case holds one sample");
            assert!(
                (code - expected).abs() < f64::EPSILON,
                "{samples:?} widened to {code} rather than {expected}"
            );
            assert_eq!(samples.code(1), None, "{samples:?}");
        }
    }

    #[test]
    fn no_type_in_this_set_has_an_input_or_output_method() {
        // #31 asks for this in its done-condition, and it is the clause that
        // cannot be asserted by calling something: what has to be shown is that
        // a method is ABSENT. So the subject is the source of the two modules
        // that hold these types, read at compile time.
        //
        // What it refuses is the standard library's four doors to the outside:
        // files, the process environment, the network, and paths. A type that
        // reaches any of them has stopped being what a reader produces and has
        // started being a reader, which is the accumulation 0008 keeps out of
        // the core when it puts every binary format in a component the core does
        // not depend on.
        //
        // A greppable rule, and the invariant lint in #23 is where rules of this
        // shape become one refusal rather than one test per module. Until that
        // exists this test is the whole of what stands against it.
        // The names are assembled rather than written out. This test reads its
        // own file, so a literal here would be a match against itself and the
        // test would refuse the tree on the first run for the wrong reason.
        const DOORS: [&str; 4] = ["fs", "io", "net", "path"];

        // Named one by one rather than walked, because a directory walk is a
        // filesystem read and this is a unit test in a suite that may not have
        // the tree beside it.
        let sources: [(&str, &str); 2] = [
            ("measurement.rs", include_str!("measurement.rs")),
            ("unit.rs", include_str!("unit.rs")),
        ];

        for (name, source) in sources {
            for door in DOORS {
                let needle = format!("std::{door}");
                assert!(
                    !source.contains(&needle),
                    "{name} names {needle}, so a type in this set reaches outside itself"
                );
            }
        }
    }
}
