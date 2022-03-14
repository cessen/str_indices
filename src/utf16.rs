//! Index by utf16 code units.

use crate::byte_chunk::{ByteChunk, Chunk};

/// Counts the utf16 code units that would be in a string slice if it
/// were encoded as utf16.
///
/// Runs in O(N) time.
#[inline]
pub fn count(text: &str) -> usize {
    crate::chars::count(text) + count_surrogates_internal::<Chunk>(text.as_bytes())
}

/// Counts the utf16 surrogate pairs that would be in a string slice if
/// it were encoded as utf16.
///
/// Runs in O(N) time.
#[inline]
pub fn count_surrogates(text: &str) -> usize {
    count_surrogates_internal::<Chunk>(text.as_bytes())
}

/// Converts from byte-index to utf16-code-unit-index in a string slice.
///
/// If the byte is in the middle of a multi-byte char, returns the utf16
/// index of the char that the byte belongs to.
///
/// Any past-the-end index will return the one-past-the-end utf16 index.
///
/// Runs in O(N) time.
#[inline]
pub fn from_byte_idx(text: &str, byte_idx: usize) -> usize {
    crate::chars::from_byte_idx(text, byte_idx)
        + count_surrogates_internal::<Chunk>(&text.as_bytes()[..byte_idx.min(text.len())])
}

/// Converts from utf16-code-unit-index to byte-index in a string slice.
///
/// If the utf16 index is in the middle of a char, returns the bytes
/// index of the char that utf16 code unit belongs to.
///
/// Any past-the-end index will return the one-past-the-end byte index.
///
/// Runs in O(N) time.
#[inline]
pub fn to_byte_idx(text: &str, utf16_idx: usize) -> usize {
    // TODO: optimized version.  This is pretty slow.
    let mut broke = false;
    let mut byte_i = 0;
    let mut utf16_i = 0;
    for (i, c) in text.char_indices() {
        utf16_i += c.len_utf16();
        byte_i = i;
        if utf16_i > utf16_idx {
            broke = true;
            break;
        }
    }

    if !broke {
        text.len()
    } else {
        byte_i
    }
}

//-------------------------------------------------------------

#[inline(always)]
fn count_surrogates_internal<T: ByteChunk>(text: &[u8]) -> usize {
    // We chop off the last three bytes, because all surrogate pairs are
    // four bytes in utf8, and so it prevents counting partial
    // characters.
    if text.len() <= 3 {
        return 0;
    }
    let text = &text[..(text.len() - 3)];

    // Get `middle` for more efficient chunk-based counting.
    let (start, middle, end) = unsafe { text.align_to::<T>() };

    let mut utf16_surrogate_count = 0;

    // Take care of unaligned bytes at the beginning.
    for byte in start.iter() {
        utf16_surrogate_count += ((byte & 0xf0) == 0xf0) as usize;
    }

    // Take care of the middle bytes in big chunks.
    for chunks in middle.chunks(T::MAX_ACC) {
        let mut acc = T::zero();
        for chunk in chunks.iter() {
            let tmp = chunk.bitand(T::splat(0xf0)).cmp_eq_byte(0xf0);
            acc = acc.add(tmp);
        }
        utf16_surrogate_count += acc.sum_bytes();
    }

    // Take care of unaligned bytes at the end.
    for byte in end.iter() {
        utf16_surrogate_count += ((byte & 0xf0) == 0xf0) as usize;
    }

    utf16_surrogate_count
}

//=============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 45 bytes, 27 utf16 code units.
    const TEXT: &str = "Hel🐸lo world! こん🐸にち🐸🐸は!";

    #[test]
    fn count_01() {
        assert_eq!(27, count(TEXT));
    }

    #[test]
    fn count_surrogates_01() {
        assert_eq!(4, count_surrogates(TEXT));
    }

    #[test]
    fn from_byte_idx_01() {
        assert_eq!(0, from_byte_idx(TEXT, 0));

        assert_eq!(3, from_byte_idx(TEXT, 3));
        assert_eq!(3, from_byte_idx(TEXT, 4));
        assert_eq!(3, from_byte_idx(TEXT, 5));
        assert_eq!(3, from_byte_idx(TEXT, 6));
        assert_eq!(5, from_byte_idx(TEXT, 7));

        assert_eq!(7, from_byte_idx(TEXT, 9));

        assert_eq!(17, from_byte_idx(TEXT, 23));
        assert_eq!(17, from_byte_idx(TEXT, 24));
        assert_eq!(17, from_byte_idx(TEXT, 25));
        assert_eq!(17, from_byte_idx(TEXT, 26));
        assert_eq!(19, from_byte_idx(TEXT, 27));

        assert_eq!(21, from_byte_idx(TEXT, 33));
        assert_eq!(21, from_byte_idx(TEXT, 34));
        assert_eq!(21, from_byte_idx(TEXT, 35));
        assert_eq!(21, from_byte_idx(TEXT, 36));
        assert_eq!(23, from_byte_idx(TEXT, 37));
        assert_eq!(23, from_byte_idx(TEXT, 38));
        assert_eq!(23, from_byte_idx(TEXT, 39));
        assert_eq!(23, from_byte_idx(TEXT, 40));
        assert_eq!(25, from_byte_idx(TEXT, 41));

        assert_eq!(27, from_byte_idx(TEXT, 45));
        assert_eq!(27, from_byte_idx(TEXT, 46)); // Index 1 past the end.
    }

    #[test]
    fn to_byte_idx_01() {
        assert_eq!(to_byte_idx(TEXT, 0), 0);

        assert_eq!(3, to_byte_idx(TEXT, 3));
        assert_eq!(3, to_byte_idx(TEXT, 4));
        assert_eq!(7, to_byte_idx(TEXT, 5));

        assert_eq!(9, to_byte_idx(TEXT, 7));

        assert_eq!(23, to_byte_idx(TEXT, 17));
        assert_eq!(23, to_byte_idx(TEXT, 18));
        assert_eq!(27, to_byte_idx(TEXT, 19));

        assert_eq!(33, to_byte_idx(TEXT, 21));
        assert_eq!(33, to_byte_idx(TEXT, 22));
        assert_eq!(37, to_byte_idx(TEXT, 23));
        assert_eq!(37, to_byte_idx(TEXT, 24));
        assert_eq!(41, to_byte_idx(TEXT, 25));

        assert_eq!(45, to_byte_idx(TEXT, 27));
        assert_eq!(45, to_byte_idx(TEXT, 27)); // Index 1 past the end.
    }
}
