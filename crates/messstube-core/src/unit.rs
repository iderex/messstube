//! The unit vocabulary, from #31.
//!
//! A unit is a value out of a fixed set and never a free string. The reason is
//! the downstream tool: given a string it has to guess, and two readers written
//! a month apart spell volts `V`, `Volts` and `VOLT`. A closed set makes that
//! disagreement a compile error in this repository rather than a silent
//! disagreement in somebody's analysis.
//!
//! WHAT THIS SET IS DERIVED FROM. The entries are the quantities the four
//! families in `docs/landscape.md` are recorded as producing, plus the two the
//! first format states in its own preamble:
//!
//! ```text
//! git grep -n 'XUNIT' -- docs/landscape.md
//! docs/landscape.md:148:    :WFMPRE:NR_PT 10000;...;XUNIT "s";XINCR 400.00...
//! ```
//!
//! The line is elided for width; the command prints it whole. It gives
//! seconds and volts for the oscilloscope family, metres for surface metrology,
//! volts, amperes, ohms, tesla and kelvin for the Hall rigs, and pascals, watts
//! and a temperature for the process controllers. It is not a general unit
//! library and it is not exhaustive. Adding an entry is a change somebody argues
//! for in an issue, which is the property the closed set exists for.
//!
//! PREFIXES ARE NOT UNITS HERE. An instrument stating millivolts is a channel in
//! [`Volt`](Unit::Volt) whose transform carries the factor of a thousandth, and
//! not a `Millivolt` entry. Otherwise the vocabulary grows by a factor of twenty
//! for every quantity in it, and two readers can spell the same physical
//! quantity two ways again, which is what this type exists to stop.

/// A unit of a channel or of an axis.
///
/// The set is closed on purpose, so that adding to it is a visible change to
/// this file rather than a string somebody invents inside a reader.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Unit {
    /// Volt. Oscilloscope vertical axes and the Hall families' readings.
    Volt,
    /// Ampere. The forced current in a Hall measurement.
    Ampere,
    /// Ohm. Sheet and bulk resistance.
    Ohm,
    /// Watt. Source power on a process controller.
    Watt,
    /// Second. Every time axis in scope.
    Second,
    /// Metre. Surface metrology, and the sample thickness a Hall derivation
    /// needs.
    Metre,
    /// Hertz.
    Hertz,
    /// Kelvin.
    Kelvin,
    /// Degree Celsius, which is a separate entry rather than kelvin with an
    /// offset folded into the transform. An instrument that stated Celsius said
    /// Celsius, and converting on the way in is the same move
    /// `docs/decisions/0004-what-a-read-produces.md` refuses when it keeps
    /// stored codes rather than multiplying them out.
    DegreeCelsius,
    /// Tesla. The applied field in a Hall measurement.
    Tesla,
    /// Pascal. Chamber pressure.
    Pascal,
    /// A quantity that has no unit: a ratio, a count, an index.
    Dimensionless,
    /// WHAT THE FILE SAID, WHERE THIS VOCABULARY HAS NO ENTRY FOR IT.
    ///
    /// This is not a way back to free strings and it is not the variant a
    /// reader reaches for because `Volt` was inconvenient. A reader that has
    /// volts writes [`Volt`](Unit::Volt), and one that writes them here is
    /// wrong in review.
    ///
    /// It exists because the alternative is worse. Instruments in scope state
    /// units this set has no entry for - standard cubic centimetres per minute
    /// on a gas flow is the case already recorded in `docs/landscape.md` under
    /// the process controllers - and the two ways of handling that without this
    /// variant are to discard what the instrument wrote, or to map it onto the
    /// nearest entry that fits. The first destroys the thing an archive exists
    /// to keep. The second is indistinguishable afterwards from a reader that
    /// knew.
    ///
    /// A downstream tool cannot mistake this for a unit it understands, which
    /// is the property the closed set was for. What it carries is evidence,
    /// stored exactly as the file spelled it, and an entry here appearing often
    /// enough is the argument for adding a real one above.
    NotInThisVocabulary(String),
}

impl Unit {
    /// The symbol a person expects to read, or, for a unit this vocabulary does
    /// not hold, exactly what the file said.
    ///
    /// Returns an owned string rather than a borrow. That is the first of the
    /// three constraints in `docs/decisions/0002-product-surface.md`: the public
    /// interface returns plain owned data and keeps lifetimes out of its
    /// signatures, so that a binding is an addition later rather than a
    /// redesign. The cost is an allocation on a short string.
    #[must_use]
    pub fn symbol(&self) -> String {
        match self {
            Unit::Volt => "V".to_owned(),
            Unit::Ampere => "A".to_owned(),
            Unit::Ohm => "Ohm".to_owned(),
            Unit::Watt => "W".to_owned(),
            Unit::Second => "s".to_owned(),
            Unit::Metre => "m".to_owned(),
            Unit::Hertz => "Hz".to_owned(),
            Unit::Kelvin => "K".to_owned(),
            Unit::DegreeCelsius => "degC".to_owned(),
            Unit::Tesla => "T".to_owned(),
            Unit::Pascal => "Pa".to_owned(),
            Unit::Dimensionless => String::new(),
            Unit::NotInThisVocabulary(stated) => stated.clone(),
        }
    }

    /// Whether this vocabulary holds an entry for the unit, as opposed to
    /// carrying what the file said and admitting it did not recognise it.
    ///
    /// A caller writing a metadata document needs to tell the two apart, and a
    /// caller deciding whether to convert a value must not treat the second as
    /// though the project had understood it.
    #[must_use]
    pub fn is_recognised(&self) -> bool {
        !matches!(self, Unit::NotInThisVocabulary(_))
    }
}

#[cfg(test)]
mod tests {
    use super::Unit;

    #[test]
    fn every_recognised_unit_carries_a_symbol_and_the_dimensionless_one_is_empty() {
        // Ohm is spelled `Ohm` rather than with the Greek letter, deliberately.
        // A symbol outside ASCII is the shape the Trojan Source check in this
        // tree exists to reason about, and a unit symbol is not the place to
        // spend that argument.
        assert_eq!(Unit::Volt.symbol(), "V");
        assert_eq!(Unit::Ohm.symbol(), "Ohm");
        assert_eq!(Unit::Second.symbol(), "s");
        assert_eq!(Unit::DegreeCelsius.symbol(), "degC");

        // The one entry whose symbol is empty, and it is empty rather than
        // absent because a dimensionless quantity is a stated fact about the
        // channel and not a missing unit.
        assert_eq!(Unit::Dimensionless.symbol(), "");
        assert!(Unit::Dimensionless.is_recognised());
    }

    #[test]
    fn an_unrecognised_unit_is_returned_exactly_as_the_file_spelled_it() {
        // The recorded case from `docs/landscape.md`: a gas flow on a process
        // controller, in a unit this vocabulary has no entry for. What a reader
        // must not do is round it to something it does recognise.
        let stated = Unit::NotInThisVocabulary("sccm".to_owned());

        assert_eq!(stated.symbol(), "sccm");
        assert!(!stated.is_recognised());
        assert_ne!(stated, Unit::Dimensionless);
    }
}
