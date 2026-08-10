//! The damaged file cases for the first format, from #50, and the error each
//! one has to give.
//!
//! A READER THAT ONLY HANDLES GOOD FILES IS HALF A READER, and the missing half
//! is the one somebody needs when they are rescuing an archive off a machine
//! that is going away. The cases below are the ones that actually happen to
//! these files rather than the ones that are easy to generate: a pulled cable,
//! an interrupted copy, a length field nobody sanity-checked, a concatenation
//! somebody did with `cat`, and a firmware writing a value the note did not
//! know about.
//!
//! EVERY CASE ASSERTS THE SPECIFIC ERROR AND ITS OFFSET. Asserting that some
//! error happened would pass on a reader that returned the same refusal for
//! everything, which is the reader these cases exist to distinguish from a
//! useful one. The offset is asserted for the reason
//! `docs/decisions/0006-errors-and-partial-reads.md` gives: it is the part that
//! helps a person, because they have a hex editor and not this source, and it
//! is the part most likely to be silently wrong.
//!
//! THE FIXTURES ARE ESCAPED LITERALS. Each one is a whole file written out in
//! this source, under the rule in `docs/testing.md`: they exist to carry exact
//! bytes, and a raw file in the tree is subject to whatever the checkout does
//! to line endings. Writing each fixture whole rather than deriving it from a
//! neighbour is also what makes the offsets below literal numbers somebody can
//! count to.
//!
//! WHAT THE FILES ARE. All of them are the same four-sample record, in the
//! short keyword spelling: a preamble of 131 bytes, then `:CURV #18`, then
//! eight bytes of sample codes. The whole and undamaged one is `WHOLE` and
//! every assertion about an offset is a byte of that layout.
//!
//! ONE OF THE SEVEN CASES IS NOT WHAT #50 CALLS IT, and the disagreement is
//! deliberate rather than an oversight. #50 lists "trailing bytes after a
//! complete record" as damage. `docs/formats/tektronix-isf.md` landed after it
//! and observes a real file holding two records laid end to end, with nothing
//! separating them and nothing terminating the last, and says in as many words
//! that a reader treating trailing bytes as damage refuses a real file. Both
//! halves are covered below: bytes after a record that begin another record are
//! a second channel, and bytes after a record that do not are damage located
//! where they begin. Reading the note as the authority over the older issue is
//! the whole of the disagreement.

#![forbid(unsafe_code)]
// Turned off for test code only: a test whose precondition does not hold has to
// stop loudly and say which precondition that was.
#![allow(clippy::panic, clippy::expect_used)]

use messstube_core::error::{ReadError, ReadOptions};
use messstube_core::reader::Reader as _;
use messstube_tektronix_isf::READER;

/// The whole file every case below is a damaged version of.
///
/// The preamble is 131 bytes, so `:CURV` begins at byte 131, the block length
/// is spelled at byte 139 and the eight bytes of sample codes begin at byte
/// 140. The file is 148 bytes.
const WHOLE: &[u8] = b":WFMP:NR_P 4;BYT_N 2;ENC BIN;BN_F RI;BYT_O MSB;PT_F Y;XUN \"s\";\
XIN 400.0E-9;XZE 0.0E+0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #18\x00\x00\x7f\xff\x80\x00\x01\x00";

/// Case one. A full disk or a pulled cable while the file was being written:
/// the preamble and the length are intact and the block stops early. Five of
/// the eight sample bytes are here.
const TRUNCATED_AT_THE_END: &[u8] =
    b":WFMP:NR_P 4;BYT_N 2;ENC BIN;BN_F RI;BYT_O MSB;PT_F Y;XUN \"s\";\
XIN 400.0E-9;XZE 0.0E+0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #18\x00\x00\x7f\xff\x80";

/// Case two. A copy that was interrupted, so the file stops inside the
/// preamble and there is no block marker anywhere in it. Forty bytes.
const TRUNCATED_INSIDE_THE_HEADER: &[u8] = b":WFMP:NR_P 4;BYT_N 2;ENC BIN;BN_F RI;BYT";

/// Case three. A length field larger than the file, which is the case the
/// checked allocation helper exists for. The preamble and the block agree with
/// each other on twenty million bytes and the file holds eight.
const A_BLOCK_LARGER_THAN_THE_FILE: &[u8] =
    b":WFMP:NR_P 10000000;BYT_N 2;ENC BIN;BN_F RI;BYT_O MSB;\
PT_F Y;XUN \"s\";XIN 400.0E-9;XZE 0.0E+0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #820000000\x00\x00\x7f\xff\x80\x00\x01\x00";

/// Case four. A length field of zero, against a preamble promising four
/// samples.
const A_BLOCK_LENGTH_OF_ZERO: &[u8] =
    b":WFMP:NR_P 4;BYT_N 2;ENC BIN;BN_F RI;BYT_O MSB;PT_F Y;XUN \"s\";\
XIN 400.0E-9;XZE 0.0E+0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #10\x00\x00\x7f\xff\x80\x00\x01\x00";

/// Case five, first half. Three bytes after a complete record that do not
/// begin another one.
const TRAILING_BYTES_THAT_BEGIN_NO_RECORD: &[u8] =
    b":WFMP:NR_P 4;BYT_N 2;ENC BIN;BN_F RI;BYT_O MSB;PT_F Y;XUN \"s\";\
XIN 400.0E-9;XZE 0.0E+0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #18\x00\x00\x7f\xff\x80\x00\x01\x00\x2c\x2c\x2c";

/// Case five, second half. The same trailing bytes, except that they are a
/// second complete record, which the format note observes in a real file.
const A_SECOND_RECORD_AFTER_THE_FIRST: &[u8] =
    b":WFMP:NR_P 4;BYT_N 2;ENC BIN;BN_F RI;BYT_O MSB;PT_F Y;XUN \"s\";\
XIN 400.0E-9;XZE 0.0E+0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #18\x00\x00\x7f\xff\x80\x00\x01\x00\
:WFMP:NR_P 4;BYT_N 2;ENC BIN;BN_F RI;BYT_O MSB;PT_F Y;XUN \"s\";\
XIN 400.0E-9;XZE 0.0E+0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #18\x00\x01\x00\x02\x00\x03\x00\x04";

/// Case six. A field carrying a value the format does not define, which is the
/// firmware the note did not know about. `BN_F` is documented as `RI` or `RP`
/// and this file says `RX`. It sits at byte 29, and the value is two bytes long
/// in both spellings so nothing after it has moved.
const A_CODE_FORMAT_THE_FORMAT_DOES_NOT_DEFINE: &[u8] =
    b":WFMP:NR_P 4;BYT_N 2;ENC BIN;BN_F RX;BYT_O MSB;PT_F Y;XUN \"s\";\
XIN 400.0E-9;XZE 0.0E+0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #18\x00\x00\x7f\xff\x80\x00\x01\x00";

/// Case seven. The right length with the wrong signature, which separates the
/// two error kinds on real bytes: this is somebody else's file rather than a
/// damaged one of ours. One byte differs from `WHOLE`, and the file is the same
/// length.
const THE_RIGHT_LENGTH_WITH_THE_WRONG_SIGNATURE: &[u8] =
    b":XFMP:NR_P 4;BYT_N 2;ENC BIN;BN_F RI;BYT_O MSB;PT_F Y;XUN \"s\";\
XIN 400.0E-9;XZE 0.0E+0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #18\x00\x00\x7f\xff\x80\x00\x01\x00";

/// What this reader said about a file, refusing to go on if it read it.
fn refusal(bytes: &[u8]) -> ReadError {
    let mut source = std::io::Cursor::new(bytes.to_vec());
    match READER.read(&mut source, ReadOptions::default()) {
        Err(refused) => refused,
        Ok(_) => panic!("the fixture was read rather than refused"),
    }
}

/// The three parts of a damage refusal, or a failure saying it was the other
/// kind.
fn damage(bytes: &[u8]) -> (u64, String, String) {
    match refusal(bytes) {
        ReadError::Damaged {
            reader,
            offset,
            expected,
            found,
        } => {
            assert_eq!(reader, "tektronix-isf");
            (offset, expected, found)
        }
        ReadError::NotThisFormat { reader } => {
            panic!("{reader} declined the file rather than calling it damaged")
        }
    }
}

#[test]
fn the_whole_file_these_cases_are_damaged_versions_of_reads() {
    // The near miss under all seven. Every case below is one change away from
    // this file, and a reader refusing everything would pass each of them and
    // fail here.
    let mut source = std::io::Cursor::new(WHOLE.to_vec());
    let outcome = READER
        .read(&mut source, ReadOptions::default())
        .expect("the undamaged fixture reads");
    assert!(outcome.is_complete());
    assert_eq!(outcome.measurement.channels.len(), 1);
    assert_eq!(WHOLE.len(), 148);
}

#[test]
fn a_file_truncated_at_the_end_stops_where_the_block_should_have_begun() {
    let (offset, expected, found) = damage(TRUNCATED_AT_THE_END);
    // Byte 140 is where the sample block begins in this layout.
    assert_eq!(offset, 140);
    assert_eq!(expected, "8 byte(s) for the sample block");
    assert_eq!(found, "5 byte(s) before the end of the input");
}

#[test]
fn a_file_truncated_inside_the_header_says_it_found_no_block_marker() {
    let (offset, expected, found) = damage(TRUNCATED_INSIDE_THE_HEADER);
    // The record began at byte 0 and never finished. There is no more precise
    // offset to give: the missing thing is not at any byte of this file.
    assert_eq!(offset, 0);
    assert_eq!(
        expected,
        "a sample block marker within 4096 byte(s) of a record"
    );
    assert_eq!(found, "40 byte(s) with none among them");
    assert_eq!(TRUNCATED_INSIDE_THE_HEADER.len(), 40);
}

#[test]
fn a_block_larger_than_the_file_is_refused_at_the_block_and_not_reserved() {
    let (offset, expected, found) = damage(A_BLOCK_LARGER_THAN_THE_FILE);
    // Byte 154 is where this file's longer preamble puts the block. The
    // refusal comes from the cursor bounding the window, before the checked
    // allocation helper is reached: the helper is inside that window and is
    // handed a count the window has already limited, so the guard bites at the
    // outer of the two rather than at the inner one.
    assert_eq!(offset, 154);
    assert_eq!(expected, "20000000 byte(s) for the sample block");
    assert_eq!(found, "8 byte(s) before the end of the input");
    assert_eq!(A_BLOCK_LARGER_THAN_THE_FILE.len(), 162);
}

#[test]
fn a_block_length_of_zero_contradicts_the_point_count_and_says_both() {
    let (offset, expected, found) = damage(A_BLOCK_LENGTH_OF_ZERO);
    // Byte 139 is where the length is spelled, which is the byte to change.
    assert_eq!(offset, 139);
    assert_eq!(expected, "a block of 4 sample(s) at 2 byte(s) each");
    assert_eq!(found, "a block declaring 0 byte(s)");
}

#[test]
fn trailing_bytes_that_begin_no_record_are_damage_where_they_begin() {
    let (offset, expected, found) = damage(TRAILING_BYTES_THAT_BEGIN_NO_RECORD);
    // Byte 148 is the first byte after the complete record.
    assert_eq!(offset, 148);
    assert_eq!(
        expected,
        "a sample block marker within 4096 byte(s) of a record"
    );
    assert_eq!(found, "3 byte(s) with none among them");
}

#[test]
fn trailing_bytes_that_are_a_second_record_are_a_second_channel() {
    // The half of case five that #50 does not have, and the reason the other
    // half is not simply "trailing bytes are damage". A real file holding two
    // records is observed in `docs/formats/tektronix-isf.md`, and a reader
    // refusing it would refuse a file an instrument wrote.
    let mut source = std::io::Cursor::new(A_SECOND_RECORD_AFTER_THE_FIRST.to_vec());
    let outcome = READER
        .read(&mut source, ReadOptions::default())
        .expect("two records laid end to end are one file");
    assert_eq!(outcome.measurement.channels.len(), 2);
    assert_eq!(outcome.measurement.axes.len(), 1);
    assert_eq!(A_SECOND_RECORD_AFTER_THE_FIRST.len(), 296);
}

#[test]
fn a_field_value_the_format_does_not_define_is_refused_at_that_field() {
    let (offset, expected, found) = damage(A_CODE_FORMAT_THE_FORMAT_DOES_NOT_DEFINE);
    // Byte 29 is where `BN_F RX` begins. An offset of 0 would be true of the
    // record and useless to somebody looking for the field.
    assert_eq!(offset, 29);
    assert_eq!(expected, "RI or RP as the code format");
    assert_eq!(found, "RX");
}

#[test]
fn the_right_length_with_the_wrong_signature_is_not_this_format() {
    // The case the whole error model is built around, on real code. This file
    // is the same length as a good one and differs from it in one byte, and
    // calling it damaged would send an operator looking for corruption in
    // somebody else's intact file.
    assert_eq!(THE_RIGHT_LENGTH_WITH_THE_WRONG_SIGNATURE.len(), WHOLE.len());
    match refusal(THE_RIGHT_LENGTH_WITH_THE_WRONG_SIGNATURE) {
        ReadError::NotThisFormat { reader } => assert_eq!(reader, "tektronix-isf"),
        ReadError::Damaged { offset, .. } => {
            panic!("a wrong signature was called damage at byte {offset}")
        }
    }
}

#[test]
fn no_two_of_these_cases_give_the_same_refusal() {
    // What makes the seven cases worth having. A reader returning one refusal
    // for everything passes every assertion above taken singly, and this is
    // where that reader fails: the sentence an operator reads has to
    // distinguish the file they have from the file they do not.
    let mut said: Vec<String> = Vec::new();
    for bytes in [
        TRUNCATED_AT_THE_END,
        TRUNCATED_INSIDE_THE_HEADER,
        A_BLOCK_LARGER_THAN_THE_FILE,
        A_BLOCK_LENGTH_OF_ZERO,
        TRAILING_BYTES_THAT_BEGIN_NO_RECORD,
        A_CODE_FORMAT_THE_FORMAT_DOES_NOT_DEFINE,
        THE_RIGHT_LENGTH_WITH_THE_WRONG_SIGNATURE,
    ] {
        let sentence = refusal(bytes).to_string();
        assert!(
            !said.contains(&sentence),
            "two cases give the same sentence: {sentence}"
        );
        said.push(sentence);
    }
    assert_eq!(said.len(), 7);
}
