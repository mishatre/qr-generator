//! The instance identifier and the check code of a marking record.
//!
//! A record from the consumer's system looks like this:
//!
//! ```text
//! Тип | Код/Артикул | Серия | Срок | XXXXXXXXYYYY
//!
//! XXXXXXXX — идентификатор экземпляра
//! YYYY     — визуальный/проверочный код
//! ```
//!
//! Both halves of the last field are computed here, because computing them needs a
//! hash and a random source and 1C has neither that is worth using: a BSL SHA-256
//! would be slow and hard to keep correct, and a 1C random number is not the
//! generator the consumer's existing records were made with. The record's own
//! system already issues `XXXXXXXXYYYY` in this format, and a second implementation
//! that disagrees by one character would invalidate every code already printed.
//!
//! ## Where this comes from, exactly
//!
//! Ported from the service this component replaces — its `utils.service.ts`:
//!
//! ```javascript
//! const CODE_ALPHABET = '23456789ABCDEFGHJKLMNPQRSTUVWXYZ'
//! const INSTANCE_ID_LENGTH = 8
//! const SHORT_CODE_LENGTH = 4
//!
//! export function generateInstanceId(): string {
//!     const bytes = randomBytes(5)
//!     let encoded = bytes.readUIntBE(0, 5)
//!     let result = ''
//!     for (let i = 0; i < INSTANCE_ID_LENGTH; i++) {
//!         result = CODE_ALPHABET[encoded & 0b11111] + result
//!         encoded = Math.floor(encoded / 32)
//!     }
//!     return result
//! }
//!
//! export function generateShortCode(value: string): string {
//!     const hash = createHash('sha256').update(value, 'utf8').digest()
//!     let encoded = hash.readUInt32BE(0) >>> 12
//!     let result = ''
//!     for (let i = 0; i < SHORT_CODE_LENGTH; i++) {
//!         result = CODE_ALPHABET[encoded & 0b11111] + result
//!         encoded >>>= 5
//!     }
//!     return result
//! }
//! ```
//!
//! The tests below carry vectors produced by running that code, so the port is
//! checked against the original rather than against itself.
//!
//! ## What the check code is computed over
//!
//! Not over the whole record, which is what makes it worth spelling out: it is
//! computed over the record *without* the code — everything up to and including the
//! eight character identifier — and then appended. Measured against the consumer's
//! live data:
//!
//! ```text
//! generateShortCode("2|2000020302|0085F0226|01022031|5RCD6VPE") = "AX4R"
//! 2|2000020302|0085F0226|01022031|5RCD6VPE    AX4R
//! ```
//!
//! which is the last four characters of the record as it is issued. So the code can
//! be recomputed from the record and compared, which is the whole point of having
//! one: a person reads `AX4R` off the printed code and a system recomputes it from
//! the data the code carries.

use sha2::{Digest, Sha256};

use crate::error::QrError;

/// The alphabet the codes are written in.
///
/// Thirty-two characters, and the ambiguous ones are missing on purpose: no `0` or
/// `O`, no `1` or `I`. A code that has to be read off a label by eye and typed in
/// cannot afford a character that is a coin toss.
///
/// The order is part of the format. The ordinal of a character is the five bits it
/// carries, so swapping two entries would change every code ever computed.
const CODE_ALPHABET: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// Characters in the instance identifier: eight, at five bits each, is forty bits
/// of randomness — about a thousand billion values.
pub const INSTANCE_ID_LENGTH: usize = 8;

/// Characters in the check code: four, at five bits each, is twenty bits — about a
/// million. Enough to make a guessed or mistyped record almost always fail, which
/// is what the code is for.
pub const CHECK_CODE_LENGTH: usize = 4;

/// A fresh instance identifier.
///
/// Five random bytes from the operating system, written as eight characters. The
/// randomness is the point: two instances of the same article must not collide, and
/// the identifier is what a person reads off the label to tell them apart.
pub fn instance_id() -> Result<String, QrError> {
    let mut bytes = [0u8; 5];

    getrandom::fill(&mut bytes).map_err(|error| {
        QrError::internal(format!(
            "the operating system refused to supply random bytes ({error}), so an instance \
             identifier cannot be issued"
        ))
    })?;

    // The five bytes read big-endian as one forty bit number, which is what
    // `readUIntBE(0, 5)` gives the original.
    let encoded = u64::from_be_bytes([0, 0, 0, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]]);

    Ok(encode_value(encoded, INSTANCE_ID_LENGTH))
}

/// The check code for the stem of a record.
///
/// The stem is the record without its code: `Тип|Код/Артикул|Серия|Срок|XXXXXXXX`.
/// See the module documentation for what that does and does not include — it is the
/// one thing about this format that is easy to get wrong.
pub fn check_code(stem: &str) -> String {
    let digest = Sha256::digest(stem.as_bytes());

    // The top twenty bits of the digest: four characters. `readUInt32BE(0) >>> 12`
    // in the original, which is the same twenty bits.
    let encoded = u64::from(u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) >> 12);

    encode_value(encoded, CHECK_CODE_LENGTH)
}

/// The comparison between a record and its stem is deliberately *not* here.
///
/// Stripping the last four characters, hashing what is left and comparing is one
/// line of 1C, which has `Лев`, `Прав` and `=` without help. What 1C has no usable
/// form of is a hash and a random source, so that is all this module provides: a
/// rule written out in two languages is a rule that eventually disagrees with
/// itself.
///
/// Write the low `digits * 5` bits of `value` in the alphabet, most significant
/// character first.
///
/// One helper for both codes, because they differ only in how many bits they
/// started with: eight characters are five random bytes read as a forty bit number,
/// four characters are the top twenty bits of a digest. The original writes them
/// with the same loop and a different starting number, and so does this.
///
/// The loop fills the buffer from the end because the first character written is the
/// least significant five bits and it belongs last: the original prepends one
/// character per iteration with `result = CODE_ALPHABET[...] + result`, and this is
/// that loop without the quadratic string building.
fn encode_value(mut value: u64, digits: usize) -> String {
    let mut characters = vec![0u8; digits];

    for index in (0..digits).rev() {
        characters[index] = CODE_ALPHABET[(value & 0b1_1111) as usize];
        value >>= 5;
    }

    String::from_utf8(characters).expect("the alphabet is ASCII")
}

#[cfg(test)]
mod tests {
    use super::{check_code, instance_id, CHECK_CODE_LENGTH, CODE_ALPHABET, INSTANCE_ID_LENGTH};

    /// The live record from the consumer's system, as its own generator issues it,
    /// and the stem its code is computed over.
    const LIVE_RECORD: &str = "2|2000020302|0085F0226|01022031|5RCD6VPEAX4R";
    const LIVE_STEM: &str = "2|2000020302|0085F0226|01022031|5RCD6VPE";

    /// Vectors from running the JavaScript these functions are ported from.
    ///
    /// The point of the port is that it agrees with the records the consumer has
    /// already issued, so the test has to compare against the original rather than
    /// against a second reading of the same specification.
    #[test]
    fn the_check_code_matches_the_reference_implementation() {
        for (stem, expected) in [
            ("", "WGSE"),
            ("a", "TCDS"),
            ("ORDER-2026-0001", "6PFM"),
            (LIVE_STEM, "AX4R"),
            ("2|2000020302|0085F0226|01022031|5RCD6VPEAX4R", "NK5F"),
            (
                "RUNZE-%20DC-032-1/4-28|124021-0094И68-06 %200124-00184Й42-1|09.03.2027|1",
                "EHY7",
            ),
            ("The quick brown fox jumps over the lazy dog", "UYNH"),
        ] {
            assert_eq!(
                check_code(stem),
                expected,
                "the check code for {stem:?} disagrees with the service that issues these records"
            );
        }
    }

    /// The same vectors for the instance identifier, with the random bytes supplied.
    #[test]
    fn the_instance_id_encoder_matches_the_reference_implementation() {
        for (bytes, expected) in [
            ([0x00, 0x00, 0x00, 0x00, 0x00], "22222222"),
            ([0xff, 0xff, 0xff, 0xff, 0xff], "ZZZZZZZZ"),
            ([0x01, 0x23, 0x45, 0x67, 0x89], "26KNCTWB"),
            ([0xde, 0xad, 0xbe, 0xef, 0x00], "VUQVXVS2"),
            ([0x80, 0x00, 0x00, 0x00, 0x00], "J2222222"),
        ] {
            let encoded =
                u64::from_be_bytes([0, 0, 0, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]]);

            assert_eq!(
                super::encode_value(encoded, INSTANCE_ID_LENGTH),
                expected,
                "the instance identifier for {bytes:02x?} disagrees with the service"
            );
        }
    }

    /// The property the whole format rests on: the code a record carries is the one
    /// its stem produces.
    ///
    /// The rule is written out here rather than called through a helper, because the
    /// helper is the BSL wrapper's job: this is the evidence that the format is what
    /// the wrapper assumes, not a second definition of it.
    #[test]
    fn a_record_from_the_consumer_verifies_against_its_own_stem() {
        let carried = &LIVE_RECORD[LIVE_RECORD.len() - CHECK_CODE_LENGTH..];

        assert_eq!(carried, "AX4R", "the code the live record carries");
        assert_eq!(
            check_code(LIVE_STEM),
            carried,
            "and the one its stem produces"
        );
    }

    /// A single changed character has to fail, or the code is not a check.
    #[test]
    fn a_record_with_one_character_changed_does_not_verify() {
        // The same record with its last character changed, stem unchanged: the code
        // that stem produces is not the one the record now carries.
        let stem = "2|2000020302|0085F0226|01022031|5RCD6VPE";
        let tampered = "2|2000020302|0085F0226|01022031|5RCD6VPEAX4S";

        assert_ne!(
            check_code(stem),
            &tampered[tampered.len() - CHECK_CODE_LENGTH..]
        );
    }

    /// The stem is what is hashed, so a change to the stem has to change the code —
    /// including a change to the identifier, which is the last thing in the stem.
    #[test]
    fn changing_the_stem_changes_the_code() {
        assert_ne!(
            check_code("2|2000020302|0085F0226|01022031|5RCD6VPE"),
            check_code("2|2000020302|0085F0226|01022031|5RCD6VPF")
        );
        assert_ne!(
            check_code("2|2000020302|0085F0226|01022031|5RCD6VPE"),
            check_code("3|2000020302|0085F0226|01022031|5RCD6VPE")
        );
    }

    #[test]
    fn an_identifier_is_eight_characters_of_the_alphabet() {
        for _ in 0..200 {
            let id = instance_id().expect("random bytes");

            assert_eq!(id.chars().count(), INSTANCE_ID_LENGTH);
            assert!(
                id.bytes()
                    .all(|character| super::CODE_ALPHABET.contains(&character)),
                "{id} is not written in the code alphabet"
            );
        }
    }

    #[test]
    fn identifiers_do_not_collide() {
        // Not a proof of randomness — the operating system's generator is the one
        // that has to be right — but a check that the bytes reach the alphabet at
        // all rather than, say, being dropped on the floor by a broken shuffle.
        let mut seen = std::collections::HashSet::new();

        for _ in 0..2_000 {
            seen.insert(instance_id().expect("random bytes"));
        }

        assert_eq!(
            seen.len(),
            2_000,
            "an identifier repeated within two thousand draws"
        );
    }

    #[test]
    fn the_alphabet_leaves_out_the_characters_people_misread() {
        for missing in ['0', 'O', '1', 'I'] {
            assert!(
                !CODE_ALPHABET.contains(&(missing as u8)),
                "{missing} has to stay out of the alphabet"
            );
        }

        assert_eq!(
            CODE_ALPHABET.len(),
            32,
            "five bits need exactly thirty-two characters"
        );
    }

    #[test]
    fn a_stem_shorter_than_the_code_is_still_hashed_rather_than_refused() {
        // Nothing here validates the record: the wrapper decides what a record is,
        // and this function hashes whatever stem it is handed. A three character
        // record therefore splits into an empty stem and itself as the "code", which
        // cannot match — and that is the wrapper's answer to give, not this one's.
        for short in ["", "1", "12", "123"] {
            assert_eq!(check_code(short).chars().count(), CHECK_CODE_LENGTH);
        }

        assert_ne!(check_code("123"), "123");
    }

    /// Cyrillic in the stem is a real case: the consumer's article codes hold it, and
    /// the vectors above include one. The hash is over the UTF-8 bytes, which is what
    /// the original's `.update(value, 'utf8')` does.
    #[test]
    fn a_cyrillic_stem_is_hashed_as_utf8() {
        assert_eq!(check_code("Заказ №42"), check_code("Заказ №42"));
        assert_ne!(check_code("Заказ №42"), check_code("Заказ №43"));
    }
}
