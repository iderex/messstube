//! The helpers reader code reads through, from #35, implementing
//! `docs/decisions/0007-hostile-input-budget.md`.
//!
//! ONE IMPLEMENTATION RATHER THAN A RULE EVERY AUTHOR REMEMBERS. The budget a
//! reader may spend on hostile bytes is fixed once and centrally, because a rule
//! argued per reader is a rule the twelfth reader does not have. These are what
//! that budget looks like as code: a cursor that cannot read past the end, an
//! allocation that is checked against the file before anything is reserved, a
//! depth guard that refuses at a named bound, and a string reader that will not
//! run to the end of the file looking for a terminator.
//!
//! NOTHING HERE PANICS ON ANY INPUT. A reader is handed bytes by somebody it has
//! never met, and a panic on those bytes ends the process of whoever called the
//! library rather than telling them what is wrong with their file. Every
//! refusal is a [`ReadError::Damaged`] carrying an absolute offset, which is
//! what `docs/decisions/0006-errors-and-partial-reads.md` requires of every
//! damage error.
//!
//! THE OFFSET IS ABSOLUTE IN THE FILE AND STAYS ABSOLUTE. A cursor made over
//! part of a file with [`Cursor::window`] carries where that part started, so a
//! reader that descends into a block still reports offsets the person with a hex
//! editor can type in. An offset relative to a structure only the reader knows
//! about is an offset that does not help anybody.
//!
//! WHAT THE PROOFS ARE. Each guard below has a fixture that trips it and a
//! one-change near miss that does not, in [`tests`]. The near miss is the half
//! that matters: a guard that refuses everything passes its own test and breaks
//! every real file, and it is the failure mode a bound written in a hurry
//! actually has.
//!
//! NOTHING IN THIS MODULE STOPS A READER GOING AROUND IT, so the refusal comes
//! from the other side. [`Cursor::reserve`] is the only path here from a count
//! in a file to a reserved allocation, and a reader crate could still write
//! `Vec::with_capacity` over a number it read itself, or decode a fixed-width
//! number out of a slice it took. Both are refused by
//! `crates/messstube-core/tests/reader_invariants.rs`, which reads every source
//! file under `crates/readers/` on every run of the suite. That target prints
//! how many files it examined, and today the number is zero, because the first
//! reader crate is #48.
//!
//! ON LIFETIMES. [`Cursor`] borrows the bytes it reads, and the three
//! constraints in `docs/decisions/0002-product-surface.md` ask the public
//! interface to keep lifetimes out of its signatures. Those constraints are
//! about what a caller receives across a language binding: a measurement, an
//! error, a description. This is what a reader inside this repository parses
//! with, no binding ever holds one, and everything it produces is plain owned
//! data. Copying a file into the cursor to avoid naming a lifetime would buy
//! nothing and cost a second copy of every file read.

use crate::error::ReadError;

/// Which end of a fixed-width number comes first.
///
/// A value rather than a method per order, so that a reader whose byte order is
/// decided by a flag in its own header reads that flag into one of these and
/// passes it, instead of branching at every field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ByteOrder {
    /// Least significant byte first.
    Little,
    /// Most significant byte first.
    Big,
}

/// A text field as the file held it.
///
/// Two states, because these formats carry both. A field that is valid text is
/// text; a field that is not stays as the bytes it was.
///
/// THE BYTES ARE NEVER REPAIRED. Replacing an invalid sequence with a
/// replacement character produces a string that looks like a reading of the
/// file and is not one, and it destroys the evidence of what was actually
/// there. An operator name written in a code page nobody recorded is a real
/// case, and the answer to it is to hand back what was stored.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    /// The field was valid text.
    Text(String),
    /// The field was not valid text, and these are its bytes.
    Bytes(Vec<u8>),
}

impl Field {
    /// Decide which of the two a run of bytes is, without repairing either.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Field {
        match core::str::from_utf8(bytes) {
            Ok(text) => Field::Text(text.to_owned()),
            Err(_) => Field::Bytes(bytes.to_vec()),
        }
    }

    /// The text, where it was text.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Field::Text(text) => Some(text),
            Field::Bytes(_) => None,
        }
    }

    /// The bytes, whichever it is.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        match self {
            Field::Text(text) => text.as_bytes(),
            Field::Bytes(bytes) => bytes,
        }
    }
}

/// A reader's view of the bytes it was given, which cannot leave them.
///
/// Every read is checked against what remains, and a read that would go past the
/// end is a refusal at the offset where it was attempted rather than a panic
/// somewhere inside an index expression.
#[derive(Debug, Clone)]
pub struct Cursor<'bytes> {
    /// The reader's stable identifier, so that a refusal from here is already a
    /// complete error and no caller has to remember to attach it.
    reader: String,
    /// Where these bytes start in the file, so offsets stay absolute.
    base: u64,
    bytes: &'bytes [u8],
    at: usize,
}

impl<'bytes> Cursor<'bytes> {
    /// A cursor over a whole file.
    #[must_use]
    pub fn new(reader: &str, bytes: &'bytes [u8]) -> Cursor<'bytes> {
        Cursor {
            reader: reader.to_owned(),
            base: 0,
            bytes,
            at: 0,
        }
    }

    /// Where the cursor is, counted from the start of the file.
    #[must_use]
    pub fn position(&self) -> u64 {
        // Both parts are bounded by the file's length, so the sum cannot
        // exceed it. Saturating rather than wrapping regardless: an offset that
        // wrapped would be a number in an error message that points somewhere
        // else in the file.
        self.base
            .saturating_add(u64::try_from(self.at).unwrap_or(u64::MAX))
    }

    /// How many bytes are left.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.at)
    }

    /// Whether nothing is left.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// A refusal at the cursor's current position.
    ///
    /// Public because a reader refusing something this module cannot judge, such
    /// as a magic number that is wrong, should refuse in the same shape and at
    /// the same offset rather than building an error of its own.
    #[must_use]
    pub fn damaged(&self, expected: &str, found: &str) -> ReadError {
        ReadError::Damaged {
            reader: self.reader.clone(),
            offset: self.position(),
            expected: expected.to_owned(),
            found: found.to_owned(),
        }
    }

    /// The next `count` bytes, or a refusal naming what was wanted.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than `count` bytes remain.
    pub fn take(&mut self, count: usize, what: &str) -> Result<&'bytes [u8], ReadError> {
        let bytes = self.bytes;
        let end = self.at.checked_add(count).ok_or_else(|| {
            self.damaged(
                what,
                "a length that does not fit in this machine's address space",
            )
        })?;
        let taken = bytes.get(self.at..end).ok_or_else(|| {
            self.damaged(
                &format!("{count} byte(s) for {what}"),
                &format!("{} byte(s) before the end of the input", self.remaining()),
            )
        })?;
        self.at = end;
        Ok(taken)
    }

    /// Move forward without reading.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than `count` bytes remain.
    pub fn skip(&mut self, count: usize, what: &str) -> Result<(), ReadError> {
        self.take(count, what).map(|_| ())
    }

    /// Move to an absolute offset in the file.
    ///
    /// A reader following an offset field is the case this exists for, and it is
    /// also the case where the offset came out of the file and may point
    /// anywhere, including back at itself.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where the offset is outside these bytes.
    pub fn go_to(&mut self, offset: u64, what: &str) -> Result<(), ReadError> {
        let inside = offset
            .checked_sub(self.base)
            .and_then(|relative| usize::try_from(relative).ok())
            .filter(|relative| *relative <= self.bytes.len())
            .ok_or_else(|| {
                self.damaged(
                    &format!("an offset inside this input for {what}"),
                    &format!("byte {offset}, and this input ends at byte {}", self.end()),
                )
            })?;
        self.at = inside;
        Ok(())
    }

    /// Where these bytes end in the file.
    #[must_use]
    pub fn end(&self) -> u64 {
        self.base
            .saturating_add(u64::try_from(self.bytes.len()).unwrap_or(u64::MAX))
    }

    /// A cursor over the next `count` bytes, which keeps its offsets absolute.
    ///
    /// This is how a reader descends into a block without losing the property
    /// that an offset in an error is one the person reading it can type into a
    /// hex editor. It also bounds the inner read: nothing reached through the
    /// window can see past the block, whatever the block claims about itself.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than `count` bytes remain.
    pub fn window(&mut self, count: usize, what: &str) -> Result<Cursor<'bytes>, ReadError> {
        let base = self.position();
        let bytes = self.take(count, what)?;
        Ok(Cursor {
            reader: self.reader.clone(),
            base,
            bytes,
            at: 0,
        })
    }

    /// Space for `count` elements of `stored_bytes_each`, checked against the
    /// file before anything is reserved.
    ///
    /// THIS IS THE ONE PATH FROM A NUMBER IN A FILE TO AN ALLOCATION. A count
    /// read out of a header is a claim by whoever wrote the file, and the claim
    /// is checked against what the file actually contains at the point it is
    /// made, not after four billion elements have been reserved. That check is
    /// the single line that removes the ordinary parser denial of service, and
    /// it is why no reader writes `Vec::with_capacity` over a number it read.
    ///
    /// `stored_bytes_each` is the width in the file rather than the width in
    /// memory. Those differ constantly in this field, and using the in-memory
    /// width would let a file promising eight-byte samples reserve twice what it
    /// can supply.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where the file promises more than it holds.
    pub fn reserve<T>(
        &self,
        count: u64,
        stored_bytes_each: usize,
        what: &str,
    ) -> Result<Vec<T>, ReadError> {
        let each = u64::try_from(stored_bytes_each).unwrap_or(u64::MAX);
        let promised = count.checked_mul(each).ok_or_else(|| {
            self.damaged(
                &format!("a believable count of {what}"),
                &format!("{count}, whose size in bytes does not fit in a 64-bit number"),
            )
        })?;
        let available = u64::try_from(self.remaining()).unwrap_or(u64::MAX);
        if promised > available {
            return Err(self.damaged(
                &format!("{promised} byte(s) for {count} {what}"),
                &format!("{available} byte(s) before the end of the input"),
            ));
        }
        let room = usize::try_from(count).map_err(|_| {
            self.damaged(
                &format!("a believable count of {what}"),
                &format!("{count}, which this machine cannot address"),
            )
        })?;
        Ok(Vec::with_capacity(room))
    }

    /// One byte.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where the input has ended.
    pub fn u8(&mut self, what: &str) -> Result<u8, ReadError> {
        self.fixed::<1>(what).map(u8::from_le_bytes)
    }

    /// One signed byte.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where the input has ended.
    pub fn i8(&mut self, what: &str) -> Result<i8, ReadError> {
        self.fixed::<1>(what).map(i8::from_le_bytes)
    }

    /// A sixteen-bit unsigned number.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than two bytes remain.
    pub fn u16(&mut self, order: ByteOrder, what: &str) -> Result<u16, ReadError> {
        self.fixed::<2>(what).map(|bytes| match order {
            ByteOrder::Little => u16::from_le_bytes(bytes),
            ByteOrder::Big => u16::from_be_bytes(bytes),
        })
    }

    /// A sixteen-bit signed number.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than two bytes remain.
    pub fn i16(&mut self, order: ByteOrder, what: &str) -> Result<i16, ReadError> {
        self.fixed::<2>(what).map(|bytes| match order {
            ByteOrder::Little => i16::from_le_bytes(bytes),
            ByteOrder::Big => i16::from_be_bytes(bytes),
        })
    }

    /// A thirty-two-bit unsigned number.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than four bytes remain.
    pub fn u32(&mut self, order: ByteOrder, what: &str) -> Result<u32, ReadError> {
        self.fixed::<4>(what).map(|bytes| match order {
            ByteOrder::Little => u32::from_le_bytes(bytes),
            ByteOrder::Big => u32::from_be_bytes(bytes),
        })
    }

    /// A thirty-two-bit signed number.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than four bytes remain.
    pub fn i32(&mut self, order: ByteOrder, what: &str) -> Result<i32, ReadError> {
        self.fixed::<4>(what).map(|bytes| match order {
            ByteOrder::Little => i32::from_le_bytes(bytes),
            ByteOrder::Big => i32::from_be_bytes(bytes),
        })
    }

    /// A sixty-four-bit unsigned number.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than eight bytes remain.
    pub fn u64(&mut self, order: ByteOrder, what: &str) -> Result<u64, ReadError> {
        self.fixed::<8>(what).map(|bytes| match order {
            ByteOrder::Little => u64::from_le_bytes(bytes),
            ByteOrder::Big => u64::from_be_bytes(bytes),
        })
    }

    /// A sixty-four-bit signed number.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than eight bytes remain.
    pub fn i64(&mut self, order: ByteOrder, what: &str) -> Result<i64, ReadError> {
        self.fixed::<8>(what).map(|bytes| match order {
            ByteOrder::Little => i64::from_le_bytes(bytes),
            ByteOrder::Big => i64::from_be_bytes(bytes),
        })
    }

    /// A single-precision floating point number.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than four bytes remain.
    pub fn f32(&mut self, order: ByteOrder, what: &str) -> Result<f32, ReadError> {
        self.fixed::<4>(what).map(|bytes| match order {
            ByteOrder::Little => f32::from_le_bytes(bytes),
            ByteOrder::Big => f32::from_be_bytes(bytes),
        })
    }

    /// A double-precision floating point number.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than eight bytes remain.
    pub fn f64(&mut self, order: ByteOrder, what: &str) -> Result<f64, ReadError> {
        self.fixed::<8>(what).map(|bytes| match order {
            ByteOrder::Little => f64::from_le_bytes(bytes),
            ByteOrder::Big => f64::from_be_bytes(bytes),
        })
    }

    /// A text field that ends at a zero byte, refusing rather than searching to
    /// the end of the input.
    ///
    /// `cap` is the most bytes the field may be. A field with no terminator
    /// inside it is a refusal at the offset the field started, because the
    /// alternative is a reader that walks the whole file looking for a zero and
    /// then reports a string megabytes long that the format never contained.
    ///
    /// The cursor is left after the terminator, so that the caller reads the
    /// next field without knowing how long this one was.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where no zero byte appears within `cap` bytes or
    /// before the input ends.
    pub fn terminated_text(&mut self, cap: usize, what: &str) -> Result<Field, ReadError> {
        let bytes = self.bytes;
        let reach = self.at.saturating_add(cap).min(bytes.len());
        let window = bytes.get(self.at..reach).unwrap_or_default();
        let length = window.iter().position(|byte| *byte == 0).ok_or_else(|| {
            self.damaged(
                &format!("a zero byte ending {what}, within {cap} byte(s)"),
                &format!("{} byte(s) with no terminator among them", window.len()),
            )
        })?;
        let text = self.take(length, what)?;
        // The terminator itself, which take() above stopped in front of.
        self.skip(1, "the terminator")?;
        Ok(Field::from_bytes(text))
    }

    /// A text field of a fixed width, ending at the first zero byte inside it.
    ///
    /// The other shape these formats use: a field padded to a fixed size, where
    /// what follows it starts at a known offset whatever the text was. The whole
    /// width is always consumed, so a shorter string does not move the fields
    /// after it.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where fewer than `width` bytes remain.
    pub fn fixed_text(&mut self, width: usize, what: &str) -> Result<Field, ReadError> {
        let taken = self.take(width, what)?;
        let end = taken
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(taken.len());
        Ok(Field::from_bytes(taken.get(..end).unwrap_or_default()))
    }

    /// A fixed-width run of bytes, as an array.
    fn fixed<const N: usize>(&mut self, what: &str) -> Result<[u8; N], ReadError> {
        let at = self.position();
        let taken = self.take(N, what)?;
        <[u8; N]>::try_from(taken).map_err(|_| ReadError::Damaged {
            reader: self.reader.clone(),
            offset: at,
            expected: format!("{N} byte(s) for {what}"),
            found: "fewer".to_owned(),
        })
    }
}

/// A bound on how deep a nested or chained structure may go.
///
/// A format whose blocks contain blocks, or whose records chain by offset, can
/// be made to recurse until the stack ends by a file that says so. That is not
/// an exotic attack: a truncated copy off a failing instrument produces a
/// self-referencing offset regularly.
///
/// REACHING THE BOUND IS A REFUSAL THAT NAMES THE BOUND. A guard that stops
/// quietly is indistinguishable from a file that ended, which is the failure
/// part three of `docs/decisions/0007-hostile-input-budget.md` exists against.
#[derive(Debug, Clone)]
pub struct DepthGuard {
    reader: String,
    /// What is being nested, in the file's own vocabulary, so the refusal says
    /// which bound was reached rather than that a bound was.
    what: String,
    bound: usize,
    depth: usize,
}

impl DepthGuard {
    /// A guard that admits `bound` levels and refuses the one after.
    #[must_use]
    pub fn new(reader: &str, what: &str, bound: usize) -> DepthGuard {
        DepthGuard {
            reader: reader.to_owned(),
            what: what.to_owned(),
            bound,
            depth: 0,
        }
    }

    /// How deep the guard currently is.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Go one level deeper, or refuse.
    ///
    /// The offset is passed in rather than held, because the guard does not read
    /// and the cursor that does is the one that knows where it is.
    ///
    /// # Errors
    ///
    /// [`ReadError::Damaged`] where the bound has been reached.
    pub fn enter(&mut self, at: u64) -> Result<(), ReadError> {
        if self.depth >= self.bound {
            return Err(ReadError::Damaged {
                reader: self.reader.clone(),
                offset: at,
                expected: format!("at most {} level(s) of {}", self.bound, self.what),
                found: format!("a {} level of {}", self.depth.saturating_add(1), self.what),
            });
        }
        self.depth = self.depth.saturating_add(1);
        Ok(())
    }

    /// Come back up one level.
    pub fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    //! Each guard tripped, and each guard's one-change near miss.
    //!
    //! The near miss is the half that matters. A cursor refusing every read, an
    //! allocation helper refusing every count and a depth guard refusing the
    //! first level all pass a test that only checks that they refused, and all
    //! three break every real file. So no refusal below is asserted without the
    //! neighbouring acceptance next to it.
    //!
    //! Fixtures are escaped byte-string literals, as `docs/testing.md` requires,
    //! so that no checkout can rewrite a byte one of them exists to carry.

    // Turned off for test code only. A test whose precondition does not hold has
    // to stop loudly and say which precondition that was. What the workspace
    // lint set denies this for is library code, which may not end the process of
    // the program that linked it.
    #![allow(clippy::panic)]

    use super::{ByteOrder, Cursor, DepthGuard, Field};
    use crate::error::ReadError;

    /// A header of the shape these formats have: a four-byte magic, a
    /// sixteen-bit count of two, and two sixteen-bit samples. Written out so
    /// that every offset a test names below can be counted by eye.
    const HEADER: &[u8] = b"\x4d\x53\x54\x42\x02\x00\x01\x00\x02\x00";

    fn cursor(bytes: &[u8]) -> Cursor<'_> {
        Cursor::new("fixture", bytes)
    }

    fn offset_of(error: &ReadError) -> Option<u64> {
        match error {
            ReadError::Damaged { offset, .. } => Some(*offset),
            ReadError::NotThisFormat { .. } => None,
        }
    }

    #[test]
    fn a_read_past_the_end_is_refused_at_the_offset_it_was_attempted() {
        // Four bytes, and a read of eight. The refusal has to name where the
        // reader was, not where the file ended, because those differ and only
        // the first tells anybody what the reader was doing.
        let mut short = cursor(b"\x4d\x53\x54\x42");
        let refused = short.u64(ByteOrder::Little, "a sample count");
        assert_eq!(
            refused.as_ref().err().and_then(offset_of),
            Some(0),
            "{refused:?}"
        );

        // The near miss: the same read one width smaller, which fits exactly.
        // A cursor that refused this would refuse every file that ends on a
        // field boundary, which is all of them.
        let mut exact = cursor(b"\x4d\x53\x54\x42");
        assert_eq!(
            exact.u32(ByteOrder::Little, "a magic number"),
            Ok(0x4254_534d)
        );
        assert!(exact.is_empty());
    }

    #[test]
    fn the_position_a_refusal_names_is_where_the_cursor_stood() {
        let mut reading = cursor(HEADER);
        assert_eq!(
            reading.u32(ByteOrder::Little, "the magic").map(|_| ()),
            Ok(())
        );
        assert_eq!(reading.position(), 4);
        assert_eq!(reading.remaining(), 6);
        let refused = reading.take(99, "the samples");
        assert_eq!(refused.err().as_ref().and_then(offset_of), Some(4));
    }

    #[test]
    fn both_byte_orders_read_the_same_bytes_as_different_numbers() {
        // The whole reason the order is a value rather than a convention. The
        // same two bytes are 1 one way round and 256 the other, and a reader
        // that guesses is a reader whose samples are wrong by a factor nobody
        // notices until they plot it.
        let mut little = cursor(b"\x01\x00");
        assert_eq!(little.u16(ByteOrder::Little, "a count"), Ok(1));
        let mut big = cursor(b"\x01\x00");
        assert_eq!(big.u16(ByteOrder::Big, "a count"), Ok(256));

        let mut float_little = cursor(b"\x00\x00\x80\x3f");
        assert_eq!(float_little.f32(ByteOrder::Little, "a scale"), Ok(1.0));
        let mut float_big = cursor(b"\x3f\x80\x00\x00");
        assert_eq!(float_big.f32(ByteOrder::Big, "a scale"), Ok(1.0));

        let mut signed = cursor(b"\xff\xff");
        assert_eq!(signed.i16(ByteOrder::Little, "a sample"), Ok(-1));
    }

    #[test]
    fn a_window_keeps_its_offsets_absolute() {
        // A reader that descends into a block and then refuses has to report
        // where the byte is in the file. Reporting where it is in the block is
        // an offset nobody outside the reader can use.
        let mut whole = cursor(HEADER);
        assert!(whole.skip(4, "the magic").is_ok());
        let Ok(mut block) = whole.window(4, "a block") else {
            panic!("a window inside the input was refused");
        };
        assert_eq!(block.position(), 4);
        assert!(block.skip(4, "the block body").is_ok());
        let refused = block.u8("one byte past the block");
        assert_eq!(refused.err().as_ref().and_then(offset_of), Some(8));

        // And the window cannot see past itself, however much the outer input
        // still holds. That is the second thing a window is for.
        assert_eq!(block.remaining(), 0);
        assert_eq!(whole.remaining(), 2);
    }

    #[test]
    fn a_count_the_file_cannot_supply_is_refused_before_anything_is_reserved() {
        // The classic denial of service, at the size it actually arrives in: a
        // thirty-two-bit count field with every bit set, in a file of ten bytes.
        let reading = cursor(HEADER);
        let refused = reading.reserve::<u16>(u64::from(u32::MAX), 2, "samples");
        assert!(refused.is_err(), "a four-billion sample claim was accepted");

        // The near miss: the count the file actually states, which the file can
        // supply. A helper refusing this would refuse every file.
        let mut reading = cursor(HEADER);
        assert!(reading.skip(4, "the magic").is_ok());
        let count = reading
            .u16(ByteOrder::Little, "a sample count")
            .unwrap_or(0);
        assert_eq!(count, 2);
        let room = reading.reserve::<u16>(u64::from(count), 2, "samples");
        assert_eq!(room.map(|held| held.len()), Ok(0));
    }

    #[test]
    fn a_count_whose_size_overflows_is_refused_rather_than_wrapping() {
        // The arithmetic itself, which is the half a bounds check written in a
        // hurry gets wrong: count times width overflows, the product comes out
        // small, and the comparison passes.
        // The numbers are chosen so that the wrapped product is SMALL. Half of
        // 2^64 elements of two bytes each is exactly 2^64 bytes, which wraps to
        // zero, and zero passes any comparison against what the file holds. A
        // check written as a multiplication and a comparison, with no overflow
        // arm, accepts this and then reserves it.
        let reading = cursor(HEADER);
        let refused = reading.reserve::<u16>(1_u64 << 63, 2, "samples");
        assert!(refused.is_err(), "an overflowing size was accepted");
    }

    #[test]
    fn an_unterminated_field_is_refused_rather_than_read_to_the_end() {
        // Sixteen bytes with no zero among them at all, which is the case where
        // the field runs off the end of the input.
        let mut running = cursor(b"abcdefghijklmnop");
        let refused = running.terminated_text(16, "an operator name");
        assert!(refused.is_err(), "an unterminated field was accepted");

        // And the case the CAP is for, which the one above does not reach: an
        // input whose only zero is well past the field. Without the cap the
        // reader walks out of the field and returns a string twenty bytes long
        // that the format never contained. The first fixture cannot show this,
        // because its cap and its length are the same number, so removing the
        // cap changes nothing about it.
        let past_the_field = b"abcdefghijklmnopqrst uvwxyz";
        let mut capped = cursor(past_the_field);
        assert!(
            capped.terminated_text(8, "an operator name").is_err(),
            "a field with its terminator outside the cap was accepted"
        );
        // The near miss: a cap that reaches the terminator, which has to read.
        let mut roomy = cursor(past_the_field);
        assert_eq!(
            roomy.terminated_text(24, "an operator name"),
            Ok(Field::Text("abcdefghijklmnopqrst".to_owned()))
        );

        // The near miss: the same field with its terminator, one byte
        // different, which has to be read.
        let mut ended = cursor(b"abcdefghijklmno\x00");
        assert_eq!(
            ended.terminated_text(16, "an operator name"),
            Ok(Field::Text("abcdefghijklmno".to_owned()))
        );
        assert!(ended.is_empty(), "the terminator was not consumed");
    }

    #[test]
    fn a_field_that_is_not_text_comes_back_as_the_bytes_it_was() {
        // A byte no code page agrees on, inside an otherwise ordinary field.
        // Replacing it would produce a string that looks like a reading of the
        // file and is not one.
        let mut odd = cursor(b"ab\xffcd\x00");
        let read = odd.terminated_text(8, "a comment");
        assert_eq!(read, Ok(Field::Bytes(b"ab\xffcd".to_vec())));
        assert_eq!(read.as_ref().map(Field::text), Ok(None));
        assert_eq!(read.map(|field| field.bytes().len()), Ok(5));
    }

    #[test]
    fn a_fixed_width_field_consumes_its_whole_width_whatever_the_text_was() {
        // The property the padded shape depends on: what follows starts where
        // the format says it does, however short the string is.
        let mut padded = cursor(b"Ch1\x00\x00\x00\x00\x00\x2a");
        assert_eq!(
            padded.fixed_text(8, "a channel name"),
            Ok(Field::Text("Ch1".to_owned()))
        );
        assert_eq!(padded.u8("the byte after"), Ok(0x2a));
    }

    #[test]
    fn nesting_past_the_bound_is_refused_and_the_refusal_names_the_bound() {
        let mut guard = DepthGuard::new("fixture", "nested blocks", 2);
        assert!(guard.enter(0).is_ok());
        assert!(guard.enter(4).is_ok());
        let refused = guard.enter(8);
        match refused {
            Err(ReadError::Damaged {
                offset, expected, ..
            }) => {
                assert_eq!(offset, 8);
                assert!(
                    expected.contains("at most 2") && expected.contains("nested blocks"),
                    "the refusal does not say which bound was reached: {expected}"
                );
            }
            other => panic!("the bound was not enforced: {other:?}"),
        }

        // The near miss: coming back up and going down again, which is what a
        // file of many shallow blocks does and which a guard counting total
        // entries rather than depth would refuse.
        guard.leave();
        assert_eq!(guard.depth(), 1);
        assert!(guard.enter(12).is_ok());
    }

    #[test]
    fn an_offset_pointing_outside_the_input_is_refused() {
        // An offset field is a number out of the file and may point anywhere,
        // including past the end and back at itself.
        let mut reading = cursor(HEADER);
        assert!(reading.go_to(4096, "a record").is_err());

        // The near miss: an offset at the very end, which is where a chain
        // legitimately stops and which an off-by-one bound refuses.
        assert!(reading.go_to(10, "a record").is_ok());
        assert!(reading.is_empty());
        assert!(reading.go_to(11, "a record").is_err());
    }
}
