//! The two writers, from #38, implementing
//! `docs/decisions/0008-output-and-interchange.md`.
//!
//! Plain text, both of them, and no dependency. 0008 fixes that the core writes
//! a delimited table for the samples and a structured text document for
//! everything else, and this module fixes what those two look like byte for
//! byte.
//!
//! THE OUTPUT IS THE SAME ON EVERY MACHINE. That is what makes a corpus test a
//! diff rather than a comparison with a tolerance, so it is a property rather
//! than a preference. Three things carry it. The line ending is `\n` and never
//! the platform's. Numbers are written by a rule this module states and tests,
//! rather than by whatever formatting a caller's locale would suggest, and
//! nothing here can take a locale because nothing in the standard library's
//! number formatting has one to take. And the order of everything is the order
//! the measurement holds, never a sort.
//!
//! FLOATING POINT ROUND TRIPS EXACTLY. A converter that loses the last bits
//! cannot be used for archiving, which is half of what the README claims, so
//! [`number`] writes the shortest text that reads back as the same value and
//! `a_written_value_reads_back_as_the_value_that_was_written` is the test that
//! holds it.
//!
//! A NAME THAT WOULD BREAK THE FILE IS REFUSED RATHER THAN REPAIRED. A channel
//! called `Ch1<tab>raw` written into a tab-delimited table silently produces an
//! extra column, and every row after the header is then misaligned against it.
//! Rewriting the name would put a name in the output that the instrument never
//! wrote. So [`sample_table`] and [`metadata_document`] refuse, and say which
//! name and which character.

use crate::measurement::{AxisShape, Measurement, Uncertainty};
use core::fmt;
use core::fmt::Write as _;

/// The delimiter of the sample table.
///
/// A tab rather than a comma, and the reason is the decimal separator. A
/// comma-delimited table of numbers written with a decimal point is the file a
/// spreadsheet opens wrongly on a machine whose locale uses the comma the other
/// way round, and the failure is silent and looks like the data. A tab cannot
/// occur inside a number this module writes, so the two questions never meet.
pub const DELIMITER: char = '\t';

/// Which numbers the table writes.
///
/// Both are reachable because 0004 keeps both and they answer different
/// questions: an archive wants the codes the instrument stored, and an analysis
/// wants the physical values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Values {
    /// The physical values, computed through each channel's transform. The
    /// default, because it is what somebody converting a file to look at it
    /// wants.
    #[default]
    Physical,
    /// The codes as the instrument stored them, untransformed.
    Stored,
}

/// Why a measurement could not be written.
///
/// One kind, because there is one thing that can go wrong here: a name carrying
/// a character that would change the shape of the output. Everything else about
/// writing a measurement is total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unwritable {
    /// What the name belongs to, in words a person reads: `channel`, `axis`.
    pub what: String,
    /// The name as the file gave it.
    pub name: String,
    /// What is wrong with it, named as a character rather than as a code point.
    pub character: String,
}

impl fmt::Display for Unwritable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "the {} named {} cannot be written: it contains {}, which would change the shape of the output",
            self.what, self.name, self.character
        )
    }
}

impl core::error::Error for Unwritable {}

/// A number, written so that reading it back gives the same value.
///
/// Plain decimal in the range a measurement usually lives in, and exponent form
/// outside it, because plain decimal for `1e-300` is three hundred characters of
/// zeroes in a column somebody has to read. Both forms are the shortest text
/// that reads back as this exact value, which is what the standard library's
/// own formatting produces, and `a_written_value_reads_back_as_the_value_that_was_written`
/// asserts it across the boundary between the two.
///
/// A value that is not a number is written as `NaN`, `inf` or `-inf`. Those read
/// back as themselves, and a reader is not permitted to invent one, so the only
/// way one reaches here is a file that stated it.
#[must_use]
pub fn number(value: f64) -> String {
    if !value.is_finite() || value == 0.0 {
        // Zero is written plainly rather than as `0e0`, and the sign of a
        // negative zero survives, because it reads back as a negative zero and
        // a writer that dropped it would be changing a value.
        return format!("{value}");
    }
    let magnitude = value.abs();
    if (1e-4..1e16).contains(&magnitude) {
        format!("{value}")
    } else {
        format!("{value:e}")
    }
}

/// The samples as a delimited table, with a header row naming each column.
///
/// One column per axis, then one per channel, and one row per sample index. A
/// channel shorter than the longest one writes an empty field rather than a
/// zero, for the reason 0006 gives about losses: a zero is indistinguishable
/// from a measurement of zero, and this field is full of measurements that are
/// legitimately zero.
///
/// # Errors
///
/// [`Unwritable`] where an axis or channel name carries the delimiter or a line
/// break.
pub fn sample_table(measurement: &Measurement, values: Values) -> Result<String, Unwritable> {
    let mut header = Vec::new();
    for axis in &measurement.axes {
        check_name("axis", &axis.name)?;
        header.push(column_name(&axis.name, &axis.unit.symbol()));
    }
    for channel in &measurement.channels {
        check_name("channel", &channel.name)?;
        let unit = match values {
            // The codes have no unit. Saying so in the header is what stops
            // somebody reading a column of codes as though it were volts.
            Values::Stored => "stored code".to_owned(),
            Values::Physical => channel.unit.symbol(),
        };
        header.push(column_name(&channel.name, &unit));
    }

    let longest_channel = measurement
        .channels
        .iter()
        .map(|channel| channel.samples.len())
        .max()
        .unwrap_or(0);
    let rows = longest_channel.max(grid(&measurement.axes));

    let mut written = String::new();
    write_row(&mut written, &header);
    for index in 0..rows {
        let mut fields = Vec::with_capacity(header.len());
        for (place, axis) in measurement.axes.iter().enumerate() {
            fields.push(axis_field(&measurement.axes, place, &axis.shape, index));
        }
        for channel in &measurement.channels {
            let value = match values {
                Values::Physical => channel.physical(index),
                Values::Stored => channel.samples.code(index),
            };
            fields.push(value.map(number).unwrap_or_default());
        }
        write_row(&mut written, &fields);
    }

    Ok(written)
}

/// Everything that is not samples, as an indented document.
///
/// Two spaces a level, `name: value` on a line, and a list entry beginning with
/// `- `. It is a shape a person reads down and a pipeline splits on the first
/// `: `, which is why values are refused rather than escaped: a document with no
/// escaping rule has no ambiguity for a reader of it to get wrong.
///
/// # Errors
///
/// [`Unwritable`] on the same names as [`sample_table`], so that a measurement
/// which writes one output writes both.
pub fn metadata_document(measurement: &Measurement) -> Result<String, Unwritable> {
    let mut written = String::new();

    written.push_str("axes:\n");
    if measurement.axes.is_empty() {
        written.push_str("  none\n");
    }
    for axis in &measurement.axes {
        check_name("axis", &axis.name)?;
        let _ = writeln!(written, "  - name: {}", axis.name);
        let _ = writeln!(written, "    unit: {}", unit_line(&axis.unit));
        let _ = writeln!(written, "    positions: {}", axis.shape.len());
        match &axis.shape {
            AxisShape::Regular { start, step, .. } => {
                let _ = writeln!(written, "    shape: regular");
                let _ = writeln!(written, "    start: {}", number(*start));
                let _ = writeln!(written, "    step: {}", number(*step));
            }
            AxisShape::Explicit(_) => {
                // The positions themselves are in the table, one per row, and
                // repeating them here would be two places to keep the same.
                let _ = writeln!(written, "    shape: a position per sample");
            }
            AxisShape::Categorical(_) => {
                let _ = writeln!(written, "    shape: labelled");
            }
        }
    }

    written.push_str("channels:\n");
    if measurement.channels.is_empty() {
        written.push_str("  none\n");
    }
    for channel in &measurement.channels {
        check_name("channel", &channel.name)?;
        let _ = writeln!(written, "  - name: {}", channel.name);
        let _ = writeln!(written, "    unit: {}", unit_line(&channel.unit));
        let _ = writeln!(written, "    samples: {}", channel.samples.len());
        let _ = writeln!(
            written,
            "    stored width in bits: {}",
            channel.samples.stored_bits()
        );
        let _ = writeln!(written, "    transform:");
        let _ = writeln!(written, "      scale: {}", number(channel.transform.scale));
        let _ = writeln!(
            written,
            "      offset: {}",
            number(channel.transform.offset)
        );
        match &channel.uncertainty {
            // Absent rather than zero. A stated uncertainty of zero and no
            // stated uncertainty are different claims, and 0004 refuses to let
            // this library make the second look like the first.
            None => {
                let _ = writeln!(written, "    uncertainty: not stated by the file");
            }
            Some(Uncertainty::Absolute(amount)) => {
                let _ = writeln!(written, "    uncertainty:");
                let _ = writeln!(written, "      kind: absolute, in the channel unit");
                let _ = writeln!(written, "      amount: {}", number(*amount));
            }
            Some(Uncertainty::Relative(fraction)) => {
                let _ = writeln!(written, "    uncertainty:");
                let _ = writeln!(
                    written,
                    "      kind: relative, as a fraction of the reading"
                );
                let _ = writeln!(written, "      amount: {}", number(*fraction));
            }
        }
    }

    written.push_str("instrument:\n");
    match &measurement.instrument {
        None => written.push_str("  not identified by the file\n"),
        Some(instrument) => {
            let named = [
                ("manufacturer", &instrument.manufacturer),
                ("model", &instrument.model),
                ("serial number", &instrument.serial),
                ("firmware", &instrument.firmware),
            ];
            for (field, value) in named {
                if let Some(value) = value {
                    let _ = writeln!(written, "  {field}: {value}");
                }
            }
        }
    }

    written.push_str("provenance:\n");
    match measurement.provenance() {
        // The honest answer for a measurement that did not come through the read
        // path. Writing an empty block would say the read path ran and found
        // nothing to record.
        None => written.push_str("  none: this measurement did not come through the read path\n"),
        Some(provenance) => {
            for (field, value) in provenance.fields() {
                let _ = writeln!(written, "  {field}: {value}");
            }
        }
    }

    Ok(written)
}

/// `name (unit)`, or the bare name where the unit has no symbol.
fn column_name(name: &str, unit: &str) -> String {
    if unit.is_empty() {
        name.to_owned()
    } else {
        format!("{name} ({unit})")
    }
}

/// What a unit is called in the document, distinguishing the vocabulary's own
/// entries from what a file stated and this project did not recognise.
fn unit_line(unit: &crate::unit::Unit) -> String {
    let symbol = unit.symbol();
    if unit.is_recognised() {
        if symbol.is_empty() {
            "none".to_owned()
        } else {
            symbol
        }
    } else {
        format!("{symbol} (stated by the file, not in this vocabulary)")
    }
}

/// How many rows the axes on their own describe.
///
/// The product of their lengths, because a two-dimensional scan has one sample
/// per pair of positions. Zero where any axis is empty, and zero for no axes at
/// all rather than the empty product, since a measurement with no axes has no
/// grid to lay anything out on.
fn grid(axes: &[crate::measurement::Axis]) -> usize {
    if axes.is_empty() {
        return 0;
    }
    axes.iter()
        .map(|axis| axis.shape.len())
        .try_fold(1_usize, usize::checked_mul)
        .unwrap_or(usize::MAX)
}

/// The position or label of one axis at one flat sample index.
///
/// Samples are laid out with the last axis varying fastest, which
/// `crates/messstube-core/src/measurement.rs` fixes, so the index into an axis
/// is the flat index divided by the product of the lengths of the axes after it,
/// taken modulo that axis's own length. A one-axis measurement is the same
/// arithmetic with an empty product of one.
///
/// Empty where the row is past the grid the axes describe, which happens where a
/// channel is longer than its axes say it should be. An empty field says the
/// axes do not place this sample, and inventing a position for it would be the
/// writer answering a question the measurement did not.
fn axis_field(
    axes: &[crate::measurement::Axis],
    place: usize,
    shape: &AxisShape,
    flat: usize,
) -> String {
    let length = shape.len();
    if length == 0 || flat >= grid(axes) {
        return String::new();
    }
    let inner: usize = axes
        .iter()
        .skip(place.saturating_add(1))
        .map(|later| later.shape.len())
        .product();
    if inner == 0 {
        return String::new();
    }
    let index = flat.wrapping_div(inner).wrapping_rem(length);
    shape
        .label(index)
        .or_else(|| shape.position(index).map(number))
        .unwrap_or_default()
}

/// Refuse a name that would change the shape of the output.
fn check_name(what: &str, name: &str) -> Result<(), Unwritable> {
    let offending = [
        (DELIMITER, "a tab"),
        ('\n', "a line feed"),
        ('\r', "a carriage return"),
    ];
    for (character, described) in offending {
        if name.contains(character) {
            return Err(Unwritable {
                what: what.to_owned(),
                name: name.to_owned(),
                character: described.to_owned(),
            });
        }
    }
    Ok(())
}

/// One row, delimiter separated, ended with a line feed and never with the
/// platform's line ending.
fn write_row(into: &mut String, fields: &[String]) {
    let mut first = true;
    for field in fields {
        if !first {
            into.push(DELIMITER);
        }
        into.push_str(field);
        first = false;
    }
    into.push('\n');
}
