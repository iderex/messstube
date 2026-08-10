//! The Tektronix ISF waveform reader, from #48. The first reader, so it is
//! also the shape every later one copies.
//!
//! WHAT IT READS IS DESCRIBED SOMEWHERE ELSE. `docs/formats/tektronix-isf.md`
//! is the format note, written before this code and from bytes and vendor
//! documentation rather than from any implementation. Every statement this
//! reader acts on is there with the word that says where it came from, and a
//! disagreement between that page and this file is a defect in one of them.
//! Nothing about the format is argued here that is not argued there.
//!
//! EVERY READ GOES THROUGH THE BOUNDED HELPERS. The cursor in
//! `messstube_core::bounded` is the only thing here that touches a byte of the
//! input, and `Cursor::reserve` is the only path from a count in the file to a
//! reserved allocation. That is part one of
//! `docs/decisions/0007-hostile-input-budget.md`, and
//! `crates/messstube-core/tests/reader_invariants.rs` is what refuses this file
//! if it stops being true.
//!
//! THE WHOLE INPUT IS TAKEN INTO MEMORY BEFORE ANYTHING IS PARSED, because the
//! cursor reads a slice. The read path in `messstube_core::read` has already
//! read the source once to hash it, so this is the second pass over bytes the
//! caller already had, and it is not a second read of the file the caller
//! opened. A format whose records are megabytes apart would be an argument for
//! a streaming cursor; this one's largest known file is two and a half
//! megabytes.
//!
//! WHAT IS DECLARED IS `Maturity::Sketched`, and that is a statement about
//! evidence rather than about effort. No file from a physical instrument has
//! been read by this code: the corpus declares none, because whether real
//! instrument files may be redistributed here is entry 2 of #1 and is open. The
//! tests below assert the parse over fixtures made in this repository, which is
//! exactly what `docs/decisions/0009-reader-maturity.md` says the sketched
//! level claims. #49 is where files and independently obtained values move it.
//!
//! ## What this reader does not do
//!
//! It does not read `ENV` records. The format note records that the one
//! envelope file it was written from contradicts the documented meaning of the
//! point count under `PT_FMT ENV`, and that one file is not enough to settle
//! which reading is right. A reader that guessed would return half or twice the
//! samples that are there, silently, so an envelope record is refused with a
//! located error naming the field.
//!
//! It does not read `ASCii` encoded blocks. None of the four files the note was
//! written from carries one, so there is nothing here to check an
//! implementation against.
//!
//! It offers no partial read. `ReadOptions` is accepted and its partial-read
//! request is not honoured: a caller that asks for one gets the same refusal a
//! caller that did not ask gets. That is a refusal rather than a truncated
//! measurement, which is the direction
//! `docs/decisions/0006-errors-and-partial-reads.md` requires when nothing is
//! offered.
//!
//! It returns one set of axes for the whole file. A file holding several
//! records is read whole, one channel per record, and the axes come from the
//! first record. Where a later record states a different point count,
//! increment, zero or horizontal unit, the read stops at that record rather
//! than placing its samples at times the file does not state. That bound comes
//! from the measurement type, which carries axes for the measurement and not
//! per channel, and it belongs to the interface review in #59 rather than to
//! this reader.
//!
//! It says nothing about the instrument. `Measurement::instrument` is left
//! absent, because no field in this format names a manufacturer, a model, a
//! serial number or a firmware version. Filling in the manufacturer from the
//! name of the format would be the reader asserting something the file does not
//! say.

#![forbid(unsafe_code)]

use messstube_core::bounded::{ByteOrder, Cursor, Field};
use messstube_core::error::{ReadError, ReadOptions, ReadOutcome};
use messstube_core::measurement::{Axis, AxisShape, Channel, Measurement, Samples, Transform};
use messstube_core::reader::{Family, Maturity, RECOGNITION_PREFIX, Reader, Source};
use messstube_core::unit::Unit;
use std::io::SeekFrom;

/// The stable identifier. It is written into the provenance block of every
/// file this reader produces, so it is chosen once and does not change.
const ID: &str = "tektronix-isf";

/// What a file of this format begins with.
///
/// Five bytes, and the format note's section on recognising a file is where
/// that number is argued: it is the shortest prefix covering both spellings of
/// the first keyword, `:WFMPRE:` and `:WFMP:`, and the SCPI abbreviation rule
/// says every accepted spelling of `WFMPRE` begins this way. A predicate of two
/// bytes would claim any file beginning with a colon.
const SIGNATURE: &[u8] = b":WFMP";

/// What begins a sample block, in the shorter of the two spellings the note
/// records. The longer one, `:CURVE`, is this followed by an `E`.
const MARKER: &[u8] = b":CURV";

/// The most bytes this reader will look through for a marker before refusing.
///
/// A bound is needed because the preamble has no length field and no
/// terminator of its own, so looking for the marker is a scan, and an
/// unbounded scan over a hostile file is the whole of the budget in
/// `docs/decisions/0007-hostile-input-budget.md` spent in one line. The number
/// is eight times the longest preamble the format note observed, 449 bytes,
/// which leaves room for a field set nobody here has seen. Whether a preamble
/// may be longer than the 512-byte recognition prefix is under "What is not
/// understood" in the note, so the bound is deliberately well clear of it.
const MOST_PREAMBLE_BYTES: usize = 4096;

/// The most records this reader will read out of one file.
///
/// One of the four files the note was written from holds two records, and
/// nothing documents an upper limit. The bound exists so that a file made of
/// many tiny records cannot turn into an unbounded number of channels, and it
/// is generous rather than tight: a file reaching it is refused with an error
/// naming the bound rather than truncated.
const MOST_RECORDS: usize = 64;

/// The reader.
///
/// A unit value with no state, which is what the interface's `Sync` bound is
/// cheap to satisfy for and what lets a registry hold it as a constant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TektronixIsf;

/// The value a registry holds.
///
/// Whichever crate links the readers together names this; a reader crate
/// cannot add itself to a registry, because the dependency runs the other way.
pub const READER: TektronixIsf = TektronixIsf;

impl Reader for TektronixIsf {
    fn id(&self) -> String {
        ID.to_owned()
    }

    fn name(&self) -> String {
        "Tektronix ISF waveform".to_owned()
    }

    fn family(&self) -> Family {
        Family::Oscilloscope
    }

    fn maturity(&self) -> Maturity {
        // Sketched, and the module documentation says what that rests on. It
        // moves when a real file and an independently obtained value arrive,
        // which is #49, and not when this code gets better.
        Maturity::Sketched
    }

    fn extensions(&self) -> Vec<String> {
        // A hint that orders an answer and never decides one. Both spellings
        // the note observed lower-case to the same word.
        vec!["isf".to_owned()]
    }

    fn recognises(&self, prefix: &[u8]) -> bool {
        prefix.starts_with(SIGNATURE)
    }

    fn read(
        &self,
        source: &mut dyn Source,
        _options: ReadOptions,
    ) -> Result<ReadOutcome, ReadError> {
        let bytes = whole(source)?;

        // The same predicate identification asked, asked again on the bytes
        // this call was handed. A caller may reach a reader without going
        // through identification at all, and a reader that assumed otherwise
        // would report somebody else's file as a damaged one of its own.
        let prefix = bytes.get(..RECOGNITION_PREFIX).unwrap_or(&bytes);
        if !self.recognises(prefix) {
            return Err(ReadError::NotThisFormat {
                reader: ID.to_owned(),
            });
        }

        let mut cursor = Cursor::new(ID, &bytes);
        let mut channels: Vec<Channel> = Vec::new();
        let mut axis: Option<Axis> = None;
        let mut layout: Option<String> = None;

        while !cursor.is_empty() {
            if channels.len() >= MOST_RECORDS {
                return Err(cursor.damaged(
                    &format!("at most {MOST_RECORDS} record(s) in one file"),
                    "a further record",
                ));
            }
            let next = record(&mut cursor)?;
            match layout.as_deref() {
                Some(first) if first != next.layout => {
                    return Err(stopped(
                        next.at,
                        &format!("a record laid out like the first, which states {first}"),
                        &next.layout,
                    ));
                }
                Some(_) => {}
                None => {
                    layout = Some(next.layout);
                    axis = Some(next.axis);
                }
            }
            channels.push(next.channel);
        }

        // `into_iter` rather than a match on the option: a file that reached
        // here has at least the five bytes of the signature, so the loop ran at
        // least once, and writing the impossible branch would mean writing a
        // panic for it.
        Ok(ReadOutcome::complete(Measurement::new(
            channels,
            axis.into_iter().collect(),
        )))
    }
}

/// A refusal at a named offset in the file.
fn stopped(offset: u64, expected: &str, found: &str) -> ReadError {
    ReadError::Damaged {
        reader: ID.to_owned(),
        offset,
        expected: expected.to_owned(),
        found: found.to_owned(),
    }
}

/// One byte, said in a way somebody with a hex editor can act on.
fn shown(byte: u8) -> String {
    if byte.is_ascii_graphic() {
        let glyph = char::from(byte);
        format!("{byte:#04x} ({glyph})")
    } else {
        format!("{byte:#04x}")
    }
}

/// The whole input, from its start.
///
/// The source is wound back first, because the interface says nothing may be
/// assumed about where the caller left it.
///
/// A source that cannot be read is reported here as a damaged file, which is
/// the wrong word for a disk that stopped answering. The two kinds a reader may
/// return do not include one for it: that distinction is
/// `messstube_core::read::ReadPathError`, one layer up, and the read path has
/// already read the source once to hash it before a reader is called, so a
/// caller going through it never arrives here.
fn whole(source: &mut dyn Source) -> Result<Vec<u8>, ReadError> {
    source.seek(SeekFrom::Start(0)).map_err(|failure| {
        stopped(
            0,
            "a source that can be wound back to its start",
            &failure.to_string(),
        )
    })?;
    let mut bytes = Vec::new();
    let read = source.read_to_end(&mut bytes).map_err(|failure| {
        stopped(
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            "the rest of the input",
            &failure.to_string(),
        )
    })?;
    let _ = read;
    Ok(bytes)
}

/// One record: a channel, the axis its samples sit on, and the statements that
/// decide that axis, kept as text so a second record can be compared against
/// the first without comparing floating point numbers.
struct Record {
    /// Where the record began in the file.
    at: u64,
    channel: Channel,
    axis: Axis,
    layout: String,
}

/// The record at the cursor, leaving the cursor on the byte after its block.
fn record(cursor: &mut Cursor<'_>) -> Result<Record, ReadError> {
    let at = cursor.position();
    let text = preamble(cursor)?;
    let preamble = Preamble {
        items: items(&text),
        at,
    };

    let count = preamble.count("NR_PT", "NR_P")?;
    let width = preamble.width()?;
    let signed = preamble.signed()?;
    let order = preamble.order()?;
    preamble.encoding_is_binary()?;
    preamble.record_is_not_an_envelope()?;

    let (declared, at_length) = block_length(cursor)?;
    // Two independent statements of one quantity, which is why they are
    // compared rather than one of them trusted. The note records that they
    // agree in all five records it was written from, and that a file where they
    // disagree is a damaged file rather than a choice between them.
    let promised = count.checked_mul(u64::try_from(width.bytes()).unwrap_or(u64::MAX));
    if promised != Some(declared) {
        return Err(stopped(
            at_length,
            &format!(
                "a block of {count} sample(s) at {} byte(s) each",
                width.bytes()
            ),
            &format!("a block declaring {declared} byte(s)"),
        ));
    }

    let room = usize::try_from(declared).map_err(|_| {
        stopped(
            at_length,
            "a block this machine can address",
            &format!("{declared} byte(s)"),
        )
    })?;
    let mut block = cursor.window(room, "the sample block")?;
    let samples = samples(&mut block, count, width, signed, order)?;

    let (axis, layout) = preamble.axis(count)?;
    let channel = preamble.channel(samples)?;
    Ok(Record {
        at,
        channel,
        axis,
        layout,
    })
}

/// The preamble of the record at the cursor, leaving the cursor on the marker
/// that begins the sample block.
fn preamble(cursor: &mut Cursor<'_>) -> Result<String, ReadError> {
    let at = cursor.position();
    let reach = cursor.remaining().min(MOST_PREAMBLE_BYTES);

    // A clone rather than a window: the marker has to be found before the
    // preamble's length is known, and a window would consume the bytes the
    // preamble is then taken from. The clone reads through the same bounded
    // cursor and can no more leave the input than the original.
    let mut probe = cursor.clone();
    let head = probe.take(reach, "a preamble")?;
    let found = head
        .windows(MARKER.len())
        .position(|run| run == MARKER)
        .ok_or_else(|| {
            cursor.damaged(
                &format!("a sample block marker within {MOST_PREAMBLE_BYTES} byte(s) of a record"),
                &format!("{reach} byte(s) with none among them"),
            )
        })?;

    let bytes = cursor.take(found, "the preamble")?;
    match Field::from_bytes(bytes) {
        Field::Text(text) => Ok(text),
        // Not repaired, and not read past. The note observes every byte of
        // every preamble it saw inside the printable range, so bytes that are
        // not text here are a statement about the file.
        Field::Bytes(_) => Err(stopped(
            at,
            "a preamble of text",
            "bytes that are not valid text",
        )),
    }
}

/// The declared length of the sample block and where it was spelled, leaving
/// the cursor on the block's first byte.
fn block_length(cursor: &mut Cursor<'_>) -> Result<(u64, u64), ReadError> {
    cursor.skip(MARKER.len(), "the sample block marker")?;

    let at_spelling = cursor.position();
    let mut byte = cursor.u8("the rest of the sample block marker")?;
    if byte == b'E' {
        byte = cursor.u8("the rest of the sample block marker")?;
    }
    if byte != b' ' {
        return Err(stopped(
            at_spelling,
            "a space after the sample block marker",
            &shown(byte),
        ));
    }

    let at_hash = cursor.position();
    let hash = cursor.u8("the number sign introducing the block length")?;
    if hash != b'#' {
        return Err(stopped(
            at_hash,
            "a number sign introducing the block length",
            &shown(hash),
        ));
    }

    let at_count = cursor.position();
    let spelled_digits = cursor.u8("the digit count of the block length")?;
    if !spelled_digits.is_ascii_digit() || spelled_digits == b'0' {
        return Err(stopped(
            at_count,
            "one digit from 1 to 9 saying how many digits the block length has",
            &shown(spelled_digits),
        ));
    }
    let digits = usize::from(spelled_digits.saturating_sub(b'0'));

    let at_length = cursor.position();
    let spelled = cursor.take(digits, "the block length")?;
    let length = decimal(spelled).ok_or_else(|| {
        stopped(
            at_length,
            &format!("{digits} decimal digit(s) giving the block length"),
            "something else",
        )
    })?;
    Ok((length, at_length))
}

/// A run of ASCII digits as a number, or nothing.
///
/// Written out rather than handed to the standard library's parser directly,
/// which accepts a leading sign and would read `+5` as five in a place the
/// format states an unsigned count.
fn decimal(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    core::str::from_utf8(bytes).ok()?.parse::<u64>().ok()
}

/// The width of a stored sample code, which the format states as a byte count
/// and this reader keeps as the two values that count may have.
///
/// A closed pair rather than a number, so that the four ways a code can be
/// read are an exhaustive match and no arm has to be written for a width the
/// documentation excludes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Width {
    One,
    Two,
}

impl Width {
    /// The width in bytes, as the file states it.
    fn bytes(self) -> usize {
        match self {
            Width::One => 1,
            Width::Two => 2,
        }
    }

    /// The width in bits, which the preamble states separately and which the
    /// documentation says changes together with the byte count.
    fn bits(self) -> u64 {
        match self {
            Width::One => 8,
            Width::Two => 16,
        }
    }
}

/// The sample codes of one block, read at the width and order the preamble
/// states.
fn samples(
    block: &mut Cursor<'_>,
    count: u64,
    width: Width,
    signed: bool,
    order: ByteOrder,
) -> Result<Samples, ReadError> {
    let what = "sample codes";
    match (width, signed) {
        (Width::One, true) => {
            let mut values = block.reserve::<i8>(count, 1, what)?;
            for _ in 0..count {
                values.push(block.i8("a sample code")?);
            }
            Ok(Samples::I8(values))
        }
        (Width::One, false) => {
            let mut values = block.reserve::<u8>(count, 1, what)?;
            for _ in 0..count {
                values.push(block.u8("a sample code")?);
            }
            Ok(Samples::U8(values))
        }
        (Width::Two, true) => {
            let mut values = block.reserve::<i16>(count, 2, what)?;
            for _ in 0..count {
                values.push(block.i16(order, "a sample code")?);
            }
            Ok(Samples::I16(values))
        }
        (Width::Two, false) => {
            let mut values = block.reserve::<u16>(count, 2, what)?;
            for _ in 0..count {
                values.push(block.u16(order, "a sample code")?);
            }
            Ok(Samples::U16(values))
        }
    }
}

/// The `name value` items of one preamble, split on `;` outside double quotes.
///
/// The quoting is honoured rather than ignored. The format note records that
/// splitting on `;` first works on all four files it was written from and
/// would be wrong on a quoted field somebody typed a semicolon into, and
/// handling it costs one boolean.
fn items(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut item = String::new();
    let mut quoted = false;
    for glyph in text.chars() {
        match glyph {
            '"' => {
                quoted = !quoted;
                item.push(glyph);
            }
            ';' if !quoted => {
                push_item(&mut found, &item);
                item.clear();
            }
            _ => item.push(glyph),
        }
    }
    push_item(&mut found, &item);
    found
}

/// One item, with the instrument path the first items carry stripped off it.
fn push_item(into: &mut Vec<(String, String)>, item: &str) {
    let trimmed = item.trim();
    if trimmed.is_empty() {
        return;
    }
    // `:WFMPRE:NR_PT 10000` and `:WFMP:BYT_N 2` carry a leading path and the
    // items after them do not. Taking what follows the second colon leaves the
    // bare name in both cases, and a quoted value carrying a colon is untouched
    // because such an item does not begin with one.
    let named = trimmed
        .strip_prefix(':')
        .and_then(|rest| rest.split_once(':'))
        .map_or(trimmed, |(_path, rest)| rest);
    match named.split_once(' ') {
        Some((name, value)) => into.push((name.trim().to_owned(), value.trim().to_owned())),
        None => into.push((named.to_owned(), String::new())),
    }
}

/// What a preamble states for one field.
enum Stated<'value> {
    /// No item carries the name under either spelling.
    Absent,
    /// One value, however many items stated it.
    Agreed(&'value str),
    /// Two items state the name and disagree.
    Disagreeing(&'value str, &'value str),
}

/// The items of one record's preamble, and where that record began.
struct Preamble {
    items: Vec<(String, String)>,
    at: u64,
}

impl Preamble {
    /// What the preamble says under either spelling of a name.
    ///
    /// A field stated twice is normal here: the note records that `NR_PT`
    /// appears as the leading item and again in its ordinary position, with the
    /// same value both times. What a reader should do when the two disagree is
    /// under "What is not understood" in the note, so this reader refuses
    /// rather than choosing one, and the refusal says both values.
    fn stated<'names>(&'names self, long: &str, short: &str) -> Stated<'names> {
        let mut first: Option<&str> = None;
        for (name, value) in &self.items {
            if !name.eq_ignore_ascii_case(long) && !name.eq_ignore_ascii_case(short) {
                continue;
            }
            match first {
                None => first = Some(value),
                Some(seen) if seen == value => {}
                Some(seen) => return Stated::Disagreeing(seen, value),
            }
        }
        first.map_or(Stated::Absent, Stated::Agreed)
    }

    /// A field the read cannot go on without.
    fn text(&self, long: &str, short: &str) -> Result<&str, ReadError> {
        match self.stated(long, short) {
            Stated::Agreed(value) => Ok(value),
            Stated::Absent => Err(stopped(
                self.at,
                &format!("a {long} field in the preamble"),
                "a preamble carrying none",
            )),
            Stated::Disagreeing(one, other) => Err(stopped(
                self.at,
                &format!("one value for {long} in the preamble"),
                &format!("{one} and {other}"),
            )),
        }
    }

    /// A field the read can do without, refusing only where it is stated twice
    /// and disagrees with itself.
    fn optional_text(&self, long: &str, short: &str) -> Result<Option<&str>, ReadError> {
        match self.stated(long, short) {
            Stated::Agreed(value) => Ok(Some(value)),
            Stated::Absent => Ok(None),
            Stated::Disagreeing(one, other) => Err(stopped(
                self.at,
                &format!("one value for {long} in the preamble"),
                &format!("{one} and {other}"),
            )),
        }
    }

    /// A stated word, compared without regard to case because the format's own
    /// abbreviation rule is written that way.
    fn word(&self, long: &str, short: &str) -> Result<String, ReadError> {
        Ok(self.text(long, short)?.trim().to_ascii_uppercase())
    }

    /// A stated count.
    fn count(&self, long: &str, short: &str) -> Result<u64, ReadError> {
        let stated = self.text(long, short)?;
        decimal(stated.as_bytes()).ok_or_else(|| {
            stopped(
                self.at,
                &format!("a whole number of samples in {long}"),
                stated,
            )
        })
    }

    /// A stated real number.
    fn number(&self, long: &str, short: &str) -> Result<f64, ReadError> {
        let stated = self.text(long, short)?;
        stated
            .parse::<f64>()
            .map_err(|_| stopped(self.at, &format!("a number in {long}"), stated))
    }

    /// A real number the format may leave out.
    fn optional_number(&self, long: &str, short: &str) -> Result<Option<f64>, ReadError> {
        match self.optional_text(long, short)? {
            None => Ok(None),
            Some(stated) => stated
                .parse::<f64>()
                .map(Some)
                .map_err(|_| stopped(self.at, &format!("a number in {long}"), stated)),
        }
    }

    /// The width of a stored code, checked against the bit count the preamble
    /// states separately.
    fn width(&self) -> Result<Width, ReadError> {
        let stated = self.count("BYT_NR", "BYT_N")?;
        let width = match stated {
            1 => Width::One,
            2 => Width::Two,
            _ => {
                return Err(stopped(
                    self.at,
                    "a sample width of 1 or 2 bytes, which is the documented range",
                    &format!("{stated}"),
                ));
            }
        };
        // The two are documented to change together, so a file where they
        // disagree has said two different things about one quantity.
        if let Some(bits) = self.optional_text("BIT_NR", "BIT_N")? {
            let stated_bits = decimal(bits.as_bytes());
            if stated_bits != Some(width.bits()) {
                return Err(stopped(
                    self.at,
                    &format!(
                        "{} bit(s) per code beside {} byte(s)",
                        width.bits(),
                        width.bytes()
                    ),
                    bits,
                ));
            }
        }
        Ok(width)
    }

    /// Whether a code is signed.
    fn signed(&self) -> Result<bool, ReadError> {
        match self.word("BN_FMT", "BN_F")?.as_str() {
            "RI" => Ok(true),
            "RP" => Ok(false),
            other => Err(stopped(self.at, "RI or RP as the code format", other)),
        }
    }

    /// Which end of a two-byte code comes first.
    ///
    /// `MSB` is read as most significant byte first. The format note records
    /// that this is inference from a measurement rather than from a document:
    /// under the other order the waveforms of the files it was written from
    /// span four hundredths of a division. `LSB` is documented and was observed
    /// in none of those files, so the arm below is untested against any file an
    /// instrument wrote.
    fn order(&self) -> Result<ByteOrder, ReadError> {
        match self.word("BYT_OR", "BYT_O")?.as_str() {
            "MSB" => Ok(ByteOrder::Big),
            "LSB" => Ok(ByteOrder::Little),
            other => Err(stopped(self.at, "MSB or LSB as the byte order", other)),
        }
    }

    /// Refuse an encoding this reader has nothing to check itself against.
    fn encoding_is_binary(&self) -> Result<(), ReadError> {
        let stated = self.word("ENCDG", "ENC")?;
        if stated == "BINARY" || stated == "BIN" {
            return Ok(());
        }
        Err(stopped(
            self.at,
            "a binary encoded block, which is what this reader implements",
            &stated,
        ))
    }

    /// Refuse an envelope record rather than guess what its point count means.
    fn record_is_not_an_envelope(&self) -> Result<(), ReadError> {
        let stated = self.word("PT_FMT", "PT_F")?;
        if stated == "Y" {
            return Ok(());
        }
        Err(stopped(
            self.at,
            "a Y record, the only point format this reader reads",
            &format!(
                "{stated}, whose point count is under \"What is not understood\" in the format note"
            ),
        ))
    }

    /// The axis the samples sit on, and the statements that decided it.
    ///
    /// The second half is text rather than numbers so that a second record can
    /// be compared against the first on what the file said, which is exact,
    /// instead of on floating point values that were parsed from it.
    fn axis(&self, count: u64) -> Result<(Axis, String), ReadError> {
        let increment = self.number("XINCR", "XIN")?;
        let zero = self.number("XZERO", "XZE")?;
        let stated_unit = self.text("XUNIT", "XUN")?;

        // Where the field is absent the term is absent. The note records
        // `PT_OFF` as zero in every record it saw and undefined in the manual
        // it was written from, so nothing is filled in for a file that states
        // none.
        let offset = self.optional_number("PT_OFF", "PT_O")?;
        let start = match offset {
            Some(stated) => increment.mul_add(-stated, zero),
            None => zero,
        };

        let positions = usize::try_from(count).map_err(|_| {
            stopped(
                self.at,
                "a point count this machine can address",
                &format!("{count}"),
            )
        })?;

        let layout = format!(
            "{count} point(s), an increment of {}, a zero of {}, a point offset of {} and a horizontal unit of {}",
            self.text("XINCR", "XIN")?,
            self.text("XZERO", "XZE")?,
            self.optional_text("PT_OFF", "PT_O")?.unwrap_or("none"),
            stated_unit
        );

        Ok((
            Axis {
                // The file names no axis. `PT_FMT Y` is the documented record
                // of samples against time, and the axis is named for what it
                // is rather than for something a field said.
                name: "time".to_owned(),
                unit: unit_of(unquoted(stated_unit)),
                shape: AxisShape::Regular {
                    start,
                    step: increment,
                    count: positions,
                },
            },
            layout,
        ))
    }

    /// The channel these samples belong to.
    fn channel(&self, samples: Samples) -> Result<Channel, ReadError> {
        let multiplier = self.number("YMULT", "YMU")?;
        let level = self.number("YOFF", "YOF")?;
        let vertical_zero = self.number("YZERO", "YZE")?;
        let stated_unit = self.text("YUNIT", "YUN")?;

        Ok(Channel {
            // The first comma-separated field of `WFID`, documented as the
            // source. A file stating no `WFID` leaves the channel unnamed
            // rather than named after its position in the file.
            name: self
                .optional_text("WFID", "WFI")?
                .map(|stated| source_of(unquoted(stated)))
                .unwrap_or_default(),
            unit: unit_of(unquoted(stated_unit)),
            samples,
            // The documented conversion, rearranged into the affine form the
            // measurement type carries and NOT applied:
            //
            //     value = (code - YOFF) * YMULT + YZERO
            //           = YMULT * code + (YZERO - YMULT * YOFF)
            //
            // Keeping it beside the codes rather than multiplying them out is
            // what `docs/decisions/0004-what-a-read-produces.md` requires, and
            // it is why a clipped code is still recognisable afterwards.
            transform: Transform {
                scale: multiplier,
                offset: multiplier.mul_add(-level, vertical_zero),
            },
            // The format states none. Nothing here invents one, because an
            // invented uncertainty is indistinguishable from evidence.
            uncertainty: None,
        })
    }
}

/// A quoted value with its quotes taken off, and anything else untouched.
fn unquoted(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(value)
}

/// The source field of a waveform identifier.
fn source_of(wfid: &str) -> String {
    wfid.split(',').next().unwrap_or_default().trim().to_owned()
}

/// The unit vocabulary's entry for what the file spelled, or what the file
/// spelled.
///
/// The symbols are compared exactly. Only `s` and `V` were observed in the
/// files the format note was written from; the others are here because they are
/// spelled in this vocabulary the same way an instrument would spell them, and
/// a file stating anything else keeps its own spelling rather than being
/// rounded to the nearest entry.
fn unit_of(stated: &str) -> Unit {
    match stated.trim() {
        "s" => Unit::Second,
        "V" => Unit::Volt,
        "A" => Unit::Ampere,
        "Hz" => Unit::Hertz,
        "W" => Unit::Watt,
        "Ohm" => Unit::Ohm,
        other => Unit::NotInThisVocabulary(other.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    //! What these tests assert is THE PARSE, and not the numbers.
    //!
    //! `docs/testing.md` fixes the difference: an expected value is
    //! independently obtained when it came from vendor software or from an
    //! implementation not derived from this repository. Nothing here is. The
    //! fixtures below were made in this repository from
    //! `docs/formats/tektronix-isf.md`, and the values asserted against them
    //! were worked out from the same page, so they are a record of this reader
    //! agreeing with this project's own reading of the format. That is exactly
    //! what the sketched maturity level claims and it is why this reader
    //! declares it. #49 is where files an instrument wrote and values obtained
    //! independently arrive, and it is the issue that can move the level.
    //!
    //! The fixtures are escaped literals in the source, under the rule in
    //! `docs/testing.md`: they exist to carry exact bytes, and a raw file in
    //! the tree is subject to whatever the checkout does to line endings.

    // Turned off for test code only: a test whose precondition does not hold
    // has to stop loudly and say which precondition that was, and the library
    // itself may not end the process of the program that linked it.
    #![allow(clippy::expect_used, clippy::panic)]

    use super::{READER, TektronixIsf};
    use messstube_core::error::{ReadError, ReadOptions, ReadOutcome};
    use messstube_core::measurement::{AxisShape, Samples, Transform};
    use messstube_core::reader::{Maturity, Reader as _};
    use messstube_core::unit::Unit;

    /// A whole file: one record of four sixteen-bit codes, in the short
    /// keyword spelling three of the note's four files use.
    ///
    /// The eight bytes after `#18` are the block: `0x0000`, `0x7fff`,
    /// `0x8000` and `0x0100`, most significant byte first, which read as 0,
    /// 32767, -32768 and 256. The two extremes are there so that a byte order
    /// read the wrong way round could not produce them.
    const ONE_RECORD: &[u8] = b":WFMP:NR_P 4;:WFMP:BYT_N 2;BIT_N 16;ENC BIN;BN_F RI;BYT_O MSB;\
WFI \"Ch1, DC coupling, 5.000V/div, 400.0ns/div, 4 points, Sample mode\";NR_P 4;PT_F Y;\
XUN \"s\";XIN 400.0E-9;XZE -8.0E-7;PT_O 0;YUN \"V\";YMU 1.5625E-4;YOF -5.0E+2;YZE 0.0E+0;\
:CURV #18\x00\x00\x7f\xff\x80\x00\x01\x00";

    /// The same record in the long keyword spelling, with the long marker that
    /// the note observed travelling with it.
    const LONG_SPELLING: &[u8] = b":WFMPRE:NR_PT 4;:WFMPRE:BYT_NR 2;BIT_NR 16;ENCDG BINARY;\
BN_FMT RI;BYT_OR MSB;WFID \"Ch1, DC coupling, 5.000V/div, 400.0ns/div, 4 points, Sample mode\";\
NR_PT 4;PT_FMT Y;XUNIT \"s\";XINCR 400.0E-9;XZERO -8.0E-7;PT_OFF 0;YUNIT \"V\";YMULT 1.5625E-4;\
YOFF -5.0E+2;YZERO 0.0E+0;:CURVE #18\x00\x00\x7f\xff\x80\x00\x01\x00";

    /// Read a whole file through the reader, from bytes in memory and with no
    /// filesystem.
    fn read(bytes: &[u8]) -> Result<ReadOutcome, ReadError> {
        let mut source = std::io::Cursor::new(bytes.to_vec());
        READER.read(&mut source, ReadOptions::default())
    }

    /// The refusal a fixture produced, or a failure saying it was read.
    fn refusal(bytes: &[u8]) -> ReadError {
        match read(bytes) {
            Err(refused) => refused,
            Ok(_) => panic!("the fixture was read rather than refused"),
        }
    }

    /// Approximate equality with a stated tolerance, because these assertions
    /// are about arithmetic on floating point numbers.
    fn close(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-15
    }

    /// A fixture with one run of bytes swapped for another.
    ///
    /// It works on bytes rather than on text on purpose. A fixture here carries
    /// a binary block, so turning one into a string to edit its preamble fails
    /// on the sample codes, and an editing step that has to be told to ignore
    /// part of the fixture would be editing something other than the fixture.
    fn replacing(bytes: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
        let at = bytes
            .windows(from.len())
            .position(|run| run == from)
            .expect("the fixture carries the run being replaced");
        let after = at.checked_add(from.len()).expect("the fixture is small");
        let mut made = Vec::new();
        made.extend_from_slice(bytes.get(..at).expect("a prefix of the fixture"));
        made.extend_from_slice(to);
        made.extend_from_slice(bytes.get(after..).expect("a suffix of the fixture"));
        made
    }

    #[test]
    fn the_predicate_claims_both_keyword_spellings_and_declines_a_shorter_prefix() {
        // The five bytes are argued in the format note's section on
        // recognising a file: they are the shortest prefix covering
        // `:WFMPRE:` and `:WFMP:`.
        assert!(READER.recognises(b":WFMPRE:NR_PT 10000;"));
        assert!(READER.recognises(b":WFMP:NR_P 1000000;"));

        // A file that stops inside the signature is declined rather than
        // claimed, which is what keeps somebody else's damaged file from
        // becoming this reader's.
        assert!(!READER.recognises(b":WFM"));
        assert!(!READER.recognises(b""));
        assert!(!READER.recognises(b"RIGOL WFM"));
        // The predicate is anchored. A file carrying the signature somewhere
        // inside it is not one of these.
        assert!(!READER.recognises(b"xx:WFMP:NR_P 4;"));
    }

    #[test]
    fn a_reader_declares_what_the_registry_and_the_ledger_read() {
        assert_eq!(READER.id(), "tektronix-isf");
        assert_eq!(READER.name(), "Tektronix ISF waveform");
        assert_eq!(READER.extensions(), vec!["isf".to_owned()]);
        // Sketched, and it says so in code. No file an instrument wrote has
        // been read by this reader, and the level is a statement about that
        // rather than about the state of the parser.
        assert_eq!(READER.maturity(), Maturity::Sketched);
        assert_eq!(TektronixIsf, READER);
    }

    #[test]
    fn a_record_comes_back_as_a_channel_of_codes_on_a_regular_axis() {
        let outcome = read(ONE_RECORD).expect("the fixture is a whole record");
        assert!(outcome.is_complete());

        let measurement = outcome.measurement;
        assert_eq!(measurement.channels.len(), 1);
        assert_eq!(measurement.axes.len(), 1);
        // The format names no instrument, so the field is absent rather than
        // carrying the name of the format.
        assert_eq!(measurement.instrument, None);

        let channel = measurement
            .channels
            .first()
            .expect("the fixture holds one record");
        // The source field of `WFID`, which the manual documents as the first
        // of its six comma-separated fields.
        assert_eq!(channel.name, "Ch1");
        assert_eq!(channel.unit, Unit::Volt);
        assert_eq!(channel.uncertainty, None);
        assert_eq!(
            channel.samples,
            Samples::I16(vec![0, 32_767, -32_768, 256]),
            "the codes were not read most significant byte first"
        );
        assert_eq!(channel.samples.stored_bits(), 16);

        let axis = measurement.axes.first().expect("one axis was built");
        assert_eq!(axis.unit, Unit::Second);
        assert_eq!(
            axis.shape,
            AxisShape::Regular {
                start: -8.0e-7,
                step: 400.0e-9,
                count: 4,
            }
        );
    }

    #[test]
    fn the_scaling_is_recorded_as_a_transform_and_the_codes_are_left_alone() {
        // The documented conversion is
        // `value = (code - YOFF) * YMULT + YZERO`, and the measurement type
        // carries an affine transform, so the fixture's `YMULT 1.5625E-4`,
        // `YOFF -5.0E+2` and `YZERO 0.0E+0` become a scale of 1.5625e-4 and an
        // offset of 0.078125. Worked out here from the format note rather than
        // obtained independently, which is what this test asserting the parse
        // means.
        let outcome = read(ONE_RECORD).expect("the fixture is a whole record");
        let channel = outcome
            .measurement
            .channels
            .first()
            .expect("the fixture holds one record")
            .clone();

        let Transform { scale, offset } = channel.transform;
        assert!(close(scale, 1.5625e-4), "{scale}");
        assert!(close(offset, 0.078_125), "{offset}");

        // The codes are still the codes. The clipped one is recognisable
        // against the width it was stored in, which is the whole reason the
        // transform is not applied on the way out.
        let physical = channel.physical(1).expect("the channel holds four codes");
        assert!(
            close(physical, 32_767.0f64.mul_add(1.5625e-4, 0.078_125)),
            "{physical}"
        );
        assert_eq!(channel.samples.code(1), Some(32_767.0));
    }

    #[test]
    fn both_keyword_spellings_and_both_marker_spellings_read_the_same_record() {
        // The note observes one file in the long spelling and three in the
        // short one, and no file mixing them. A reader taking only one refuses
        // real files.
        let short = read(ONE_RECORD).expect("the short spelling is a whole record");
        let long = read(LONG_SPELLING).expect("the long spelling is a whole record");
        assert_eq!(short.measurement.channels, long.measurement.channels);
        assert_eq!(short.measurement.axes, long.measurement.axes);
    }

    #[test]
    fn a_file_that_does_not_begin_with_the_signature_is_not_this_format() {
        // The distinction the two error kinds exist for. This is somebody
        // else's file, not a damaged one of ours, and a caller acts on the two
        // in opposite directions.
        let refused = refusal(b"RIGOL WFM\x00\x00\x00\x00");
        assert_eq!(
            refused,
            ReadError::NotThisFormat {
                reader: "tektronix-isf".to_owned()
            }
        );
    }

    #[test]
    fn a_declared_block_length_that_contradicts_the_point_count_is_refused() {
        // Two independent statements of one quantity. The note records that
        // they agree in all five records it was written from, so a file where
        // they do not is damaged rather than a choice between them. Here the
        // block declares six bytes for four two-byte samples.
        let mut bytes = ONE_RECORD.to_vec();
        let block = bytes.len().saturating_sub(9);
        bytes.splice(block..block.saturating_add(1), *b"6");

        match refusal(&bytes) {
            ReadError::Damaged {
                offset,
                expected,
                found,
                ..
            } => {
                assert!(
                    expected.contains("4 sample(s) at 2 byte(s) each"),
                    "{expected}"
                );
                assert!(found.contains('6'), "{found}");
                // The offset is where the length was spelled, which is a byte
                // somebody can type into a hex editor.
                assert_eq!(offset, u64::try_from(block).expect("the fixture is small"));
            }
            other @ ReadError::NotThisFormat { .. } => {
                panic!("expected a damaged file: {other:?}")
            }
        }
    }

    #[test]
    fn an_envelope_record_is_refused_rather_than_guessed() {
        // The note records that the documented meaning of the point count
        // under `ENV` contradicts the one envelope file it was written from. A
        // reader that chose a reading would return half or twice the samples
        // that are there, and say nothing about it.
        let envelope = replacing(ONE_RECORD, b"PT_F Y;", b"PT_F ENV;");
        match refusal(&envelope) {
            ReadError::Damaged {
                expected, found, ..
            } => {
                assert!(expected.contains("Y record"), "{expected}");
                assert!(found.contains("ENV"), "{found}");
            }
            other @ ReadError::NotThisFormat { .. } => {
                panic!("expected a damaged file: {other:?}")
            }
        }
    }

    #[test]
    fn a_quoted_value_holding_a_semicolon_does_not_split_the_preamble() {
        // Under "What is not understood" in the format note: no observed
        // quoted value carries a `;`, and a reader that split on `;` before
        // handling quotes would be wrong on one that did. Handling it costs one
        // boolean, so it is handled.
        let awkward = replacing(ONE_RECORD, b"Sample mode", b"Sample; mode");
        let outcome = read(&awkward).expect("a quoted semicolon is not a separator");
        let channel = outcome
            .measurement
            .channels
            .first()
            .expect("the fixture holds one record");
        assert_eq!(channel.name, "Ch1");
        assert_eq!(channel.samples.len(), 4);
    }

    #[test]
    fn two_records_on_one_axis_come_back_as_two_channels() {
        // The note observes a real file holding two records laid end to end,
        // and says a reader stopping at the first block reads it correctly and
        // silently drops half of it.
        let mut both = ONE_RECORD.to_vec();
        both.extend_from_slice(ONE_RECORD);
        let outcome = read(&both).expect("two records laid end to end are one file");
        assert_eq!(outcome.measurement.channels.len(), 2);
        assert_eq!(outcome.measurement.axes.len(), 1);
    }

    #[test]
    fn a_second_record_on_a_different_axis_stops_the_read_where_it_starts() {
        // The bound the measurement type places: axes belong to the
        // measurement and not to a channel, so a second record on a different
        // time base cannot be returned beside the first. It is refused at the
        // offset the second record begins, rather than placed on the first
        // record's axis.
        let faster = replacing(ONE_RECORD, b"XIN 400.0E-9;", b"XIN 200.0E-9;");
        let mut both = ONE_RECORD.to_vec();
        both.extend_from_slice(&faster);

        match refusal(&both) {
            ReadError::Damaged {
                offset, expected, ..
            } => {
                assert_eq!(
                    offset,
                    u64::try_from(ONE_RECORD.len()).expect("the fixture is small")
                );
                assert!(expected.contains("laid out like the first"), "{expected}");
            }
            other @ ReadError::NotThisFormat { .. } => {
                panic!("expected a damaged file: {other:?}")
            }
        }
    }

    #[test]
    fn a_field_the_read_needs_is_named_when_it_is_missing() {
        let without = replacing(ONE_RECORD, b"YMU 1.5625E-4;", b"");
        match refusal(&without) {
            ReadError::Damaged { expected, .. } => {
                assert!(expected.contains("YMULT"), "{expected}");
            }
            other @ ReadError::NotThisFormat { .. } => {
                panic!("expected a damaged file: {other:?}")
            }
        }
    }

    #[test]
    fn a_field_stated_twice_and_disagreeing_with_itself_is_refused() {
        // The note records `NR_PT` stated twice with the same value in all
        // five records it saw, and lists what to do when they differ under
        // "What is not understood". This reader refuses rather than choosing
        // one of them, and says both.
        let contradictory = replacing(ONE_RECORD, b";NR_P 4;PT_F Y;", b";NR_P 5;PT_F Y;");
        match refusal(&contradictory) {
            ReadError::Damaged {
                expected, found, ..
            } => {
                assert!(expected.contains("one value for NR_PT"), "{expected}");
                assert!(found.contains('4') && found.contains('5'), "{found}");
            }
            other @ ReadError::NotThisFormat { .. } => {
                panic!("expected a damaged file: {other:?}")
            }
        }
    }

    #[test]
    fn a_preamble_with_no_marker_in_it_is_refused_at_the_bound() {
        // The scan for the marker is bounded, because the preamble carries no
        // length and no terminator of its own. A file that is all preamble is
        // refused at the bound rather than read to its end.
        let mut endless = b":WFMP:NR_P 4;".to_vec();
        endless.resize(9000, b'x');
        match refusal(&endless) {
            ReadError::Damaged {
                offset, expected, ..
            } => {
                assert_eq!(offset, 0);
                assert!(expected.contains("4096"), "{expected}");
            }
            other @ ReadError::NotThisFormat { .. } => {
                panic!("expected a damaged file: {other:?}")
            }
        }
    }

    #[test]
    fn every_prefix_of_a_whole_file_is_refused_with_an_offset_inside_it() {
        // The clause of #48 that cannot be proved by one case: every place
        // this reader can stop is a located error. Every truncation of the
        // fixture is read, and each one must either be declined as not this
        // format or refused with an offset that lies inside the bytes that were
        // offered. A panic anywhere in here would end the process of whoever
        // linked the library, which is what the lint set denies for.
        for length in 0..ONE_RECORD.len() {
            let cut = ONE_RECORD
                .get(..length)
                .expect("a prefix of a known length");
            match read(cut) {
                Ok(_) => panic!("a file cut at {length} byte(s) was read as whole"),
                Err(ReadError::NotThisFormat { .. }) => {
                    assert!(length < 5, "a file carrying the signature was declined");
                }
                Err(ReadError::Damaged { offset, .. }) => {
                    assert!(
                        offset <= u64::try_from(length).expect("the fixture is small"),
                        "a file cut at {length} byte(s) reported byte {offset}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_block_longer_than_the_file_is_refused_before_anything_is_reserved() {
        // The checked allocation helper's whole job, reached through a real
        // preamble: the point count and the declared length agree with each
        // other and both lie about the file.
        let bigger = replacing(ONE_RECORD, b":WFMP:NR_P 4;", b":WFMP:NR_P 2000000000;");
        let counted = replacing(&bigger, b";NR_P 4;PT_F Y;", b";NR_P 2000000000;PT_F Y;");
        let greedy = replacing(&counted, b":CURV #18", b":CURV #104000000000");
        match refusal(&greedy) {
            ReadError::Damaged { found, .. } => {
                assert!(found.contains("byte(s)"), "{found}");
            }
            other @ ReadError::NotThisFormat { .. } => {
                panic!("expected a damaged file: {other:?}")
            }
        }
    }
}
