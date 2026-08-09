//! The content hash the read path writes into the provenance block, from #36.
//!
//! WHY THIS IS WRITTEN OUT RATHER THAN TAKEN FROM A CRATE. `Cargo.toml` keeps
//! `[workspace.dependencies]` empty, and the sentence that keeps it empty is
//! #14's: no crate here carries a dependency that is not needed to compile an
//! empty crate. A hash is a fixed function of its input with published test
//! vectors, so the usual argument for taking a dependency, that somebody else
//! maintains the correctness, is answered here by the vectors below instead.
//! The three in [`tests`] are the published ones and the fourth is the long
//! message, so a mistake in the padding, in the length counter or in the round
//! constants reds the suite rather than producing a plausible wrong digest.
//!
//! It is SHA-256 and the name is carried beside the digest everywhere it
//! appears, in [`ContentHash`]. A hash written down without its algorithm is a
//! hash nobody can check in ten years, which is the length of time the
//! provenance block exists for.
//!
//! NOTHING HERE OPENS ANYTHING. [`digest_of`] is handed something that already
//! reads, the same way a reader is, so this module has no more reach than the
//! rest of the crate.

use core::fmt;
use std::io::Read;

/// Which hash the digest beside it was produced with.
///
/// An enumeration rather than a string, so that the name in the output comes
/// from one place and a second algorithm is a variant somebody adds rather than
/// a spelling somebody invents. There is one variant today and that is the
/// state of the tree, not a promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HashAlgorithm {
    /// SHA-256, as published in FIPS 180-4.
    Sha256,
}

impl fmt::Display for HashAlgorithm {
    /// The name as a person outside this project writes it, because the
    /// provenance block is read by somebody checking a file against a digest
    /// with whatever tool they have.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let written = match self {
            HashAlgorithm::Sha256 => "SHA-256",
        };
        formatter.write_str(written)
    }
}

/// A digest and the algorithm that produced it, never one without the other.
///
/// The digest is lower-case hexadecimal, which is what every command-line
/// checksum tool prints, so a person can compare the two by eye.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash {
    /// What produced the digest.
    pub algorithm: HashAlgorithm,
    /// The digest, lower-case hexadecimal.
    pub digest: String,
}

impl fmt::Display for ContentHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.algorithm, self.digest)
    }
}

/// Hash everything the source still has, and say how many bytes that was.
///
/// The length comes back with the digest because the read path needs both and
/// because counting it here means it is counted over exactly the bytes that were
/// hashed. Two passes over the input could disagree; one cannot.
///
/// # Errors
///
/// Whatever the source raised. This function decides nothing about a failure to
/// read; that judgement belongs to the caller, which is
/// [`read_with`](crate::read::read_with).
///
/// Generic over the source rather than taking `&mut dyn Read`, because handing
/// a `&mut dyn Source` to a parameter of that type is a trait upcast, and that
/// is not accepted by the oldest compiler `Cargo.toml` declares support for. A
/// generic costs nothing here and keeps the floor where it was set.
pub(crate) fn digest_of<R: Read + ?Sized>(
    source: &mut R,
) -> Result<(ContentHash, u64), std::io::Error> {
    let mut state = Sha256::new();
    // Eight kilobytes, which is a page or two and not tuned. The input is a
    // measurement file on somebody's disk and the hash is not the slow part of
    // reading one.
    let mut buffer = [0_u8; 8192];
    let mut length: u64 = 0;
    loop {
        let filled = source.read(&mut buffer)?;
        if filled == 0 {
            break;
        }
        let read = buffer.get(..filled).unwrap_or_default();
        state.update(read);
        length = length.saturating_add(read.len().try_into().unwrap_or(u64::MAX));
    }

    Ok((
        ContentHash {
            algorithm: HashAlgorithm::Sha256,
            digest: hex(&state.finish()),
        },
        length,
    ))
}

/// Lower-case hexadecimal, the spelling every checksum tool prints.
fn hex(bytes: &[u8; 32]) -> String {
    use fmt::Write as _;
    let mut written = String::with_capacity(64);
    for byte in bytes {
        // Writing into a `String` has no error path: `write_str` under this
        // returns `Ok` unconditionally.
        let _ = write!(written, "{byte:02x}");
    }
    written
}

/// The round constants of FIPS 180-4, the first thirty-two bits of the
/// fractional parts of the cube roots of the first sixty-four primes.
const ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// SHA-256 as a value that can be fed a byte at a time.
///
/// Streaming rather than whole-input, so that hashing a measurement file does
/// not require holding it in memory. That is the same reason the reader
/// interface takes a source rather than a vector of bytes.
struct Sha256 {
    /// The eight working words, initialised to the fractional parts of the
    /// square roots of the first eight primes.
    state: [u32; 8],
    /// The block being filled. A block is sixty-four bytes and the algorithm
    /// consumes nothing shorter.
    block: [u8; 64],
    /// How much of `block` is filled.
    filled: usize,
    /// How many bytes have been fed in, which the padding writes out at the end
    /// as a count of bits.
    length: u64,
}

impl Sha256 {
    const fn new() -> Self {
        Sha256 {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            block: [0; 64],
            filled: 0,
            length: 0,
        }
    }

    /// Feed bytes in.
    ///
    /// Indexing rather than a fallible lookup, on an array of fixed length
    /// against a counter this function is the only writer of and resets at
    /// sixty-four. A `get` here would add a branch that cannot be taken and
    /// would have to invent a behaviour for it.
    #[allow(clippy::indexing_slicing)]
    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.block[self.filled] = *byte;
            self.filled += 1;
            self.length = self.length.wrapping_add(1);
            if self.filled == 64 {
                let full = self.block;
                self.compress(&full);
                self.filled = 0;
            }
        }
    }

    /// Pad, absorb the length and hand back the digest.
    fn finish(mut self) -> [u8; 32] {
        // Captured before the padding is fed in, because feeding it moves the
        // counter and the length written out is the length of the message.
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.filled != 56 {
            self.update(&[0x00]);
        }
        self.update(&bits.to_be_bytes());

        let mut digest = [0_u8; 32];
        for (slot, word) in digest.chunks_exact_mut(4).zip(self.state) {
            slot.copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    /// One block through the compression function.
    ///
    /// The message schedule is indexed at four fixed offsets behind the word
    /// being written, which is the algorithm as published. Rewriting it through
    /// fallible lookups would hide the recurrence behind the checking, and the
    /// array is sixty-four words with a loop bounded at sixty-four.
    ///
    /// The eight working variables keep the single-letter names FIPS 180-4 gives
    /// them, against the lint that refuses single-letter bindings. Descriptive
    /// names here would make the rounds unreadable against the publication they
    /// have to be checked line by line against, which is the only way anybody
    /// verifies this function.
    #[allow(clippy::indexing_slicing, clippy::many_single_char_names)]
    fn compress(&mut self, block: &[u8; 64]) {
        let mut schedule = [0_u32; 64];
        for (slot, chunk) in schedule.iter_mut().zip(block.chunks_exact(4)) {
            let mut word: u32 = 0;
            for byte in chunk {
                word = (word << 8) | u32::from(*byte);
            }
            *slot = word;
        }
        for index in 16..64 {
            let previous = schedule[index - 15];
            let recent = schedule[index - 2];
            let mixed_low = previous.rotate_right(7) ^ previous.rotate_right(18) ^ (previous >> 3);
            let mixed_high = recent.rotate_right(17) ^ recent.rotate_right(19) ^ (recent >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(mixed_low)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(mixed_high);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for (constant, word) in ROUND_CONSTANTS.iter().zip(schedule) {
            let sum_high = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let first = h
                .wrapping_add(sum_high)
                .wrapping_add(choose)
                .wrapping_add(*constant)
                .wrapping_add(word);
            let sum_low = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let second = sum_low.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(first);
            d = c;
            c = b;
            b = a;
            a = first.wrapping_add(second);
        }

        for (slot, working) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(working);
        }
    }
}

#[cfg(test)]
mod tests {
    //! The published vectors, which are what stands in for the dependency this
    //! module was written instead of taking.
    //!
    //! Four cases, chosen for the four ways this is got wrong. The empty message
    //! is padding with no data at all. `abc` is one short block. The
    //! fifty-six-byte message is the case where the length does not fit in the
    //! block the data ended in, so the padding runs into a second one. The long
    //! message is many blocks and a length counter past a single block's worth.

    // Turned off for test code only. The library may not end the process of the
    // program that linked it, which is what the lint is for; a test that reached
    // a branch it has just proved unreachable has to stop rather than go on and
    // report a pass.
    #![allow(clippy::unreachable)]

    use super::{ContentHash, HashAlgorithm, digest_of};

    /// The digest of a byte string, through the same entry point the read path
    /// uses, so what the vectors check is the function the tree calls.
    fn digest(input: &[u8]) -> String {
        let mut source = std::io::Cursor::new(input.to_vec());
        // A cursor over memory has no failure path, and a test that swallowed
        // one would report a wrong digest as a pass.
        let Ok((hash, length)) = digest_of(&mut source) else {
            unreachable!("a cursor over bytes in memory cannot fail to read")
        };
        assert_eq!(
            length,
            u64::try_from(input.len()).unwrap_or(u64::MAX),
            "the length came back different from what went in"
        );
        assert_eq!(hash.algorithm, HashAlgorithm::Sha256);
        hash.digest
    }

    #[test]
    fn the_published_vectors_come_out() {
        assert_eq!(
            digest(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            digest(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            digest(
                b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn\
                  hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
            ),
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
        );
    }

    #[test]
    fn a_message_of_many_blocks_carries_its_length_correctly() {
        // A million bytes, which is the published long vector. It is here
        // because a length counter that overflows a block boundary wrongly, or
        // a buffer that is reset at the wrong moment, produces the right answer
        // for every short message above and the wrong one for this.
        let long = vec![b'a'; 1_000_000];
        assert_eq!(
            digest(&long),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn a_hash_is_never_written_without_the_algorithm_beside_it() {
        // The provenance block is read by somebody with a checksum tool and no
        // access to this repository. A digest on its own does not tell them
        // which tool to run.
        let hash = ContentHash {
            algorithm: HashAlgorithm::Sha256,
            digest: "e3b0c442".to_owned(),
        };
        assert_eq!(hash.to_string(), "SHA-256:e3b0c442");
        assert_eq!(HashAlgorithm::Sha256.to_string(), "SHA-256");
    }
}
