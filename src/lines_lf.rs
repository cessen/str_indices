//! Index by lines (line feed only).
//!
//! This module recognizes the following as line breaks:
//!
//! - `U+000A`          &mdash; LF (Line Feed)
//! - `U+000D` `U+000A` &mdash; CRLF (Carriage Return + Line Feed)
//!   &mdash; by coincidence due to ignoring CR.

use crate::byte_chunk::{ByteChunk, Chunk};

/// Counts the line breaks in a string slice.
///
/// Runs in O(N) time.
#[inline]
pub fn count_breaks(text: &str) -> usize {
    count_breaks_impl::<Chunk>(text.as_bytes())
}

/// Converts from byte-index to line-index in a string slice.
///
/// Line break characters are considered to be a part of the line they
/// end.  And a string that ends with a line break is considered to have
/// a final empty line.  So this function is equivalent to counting the
/// line breaks before the specified byte.
///
/// Any past-the-end index will return the last line index.
///
/// Runs in O(N) time.
#[inline]
pub fn from_byte_idx(text: &str, byte_idx: usize) -> usize {
    let i = byte_idx.min(text.len());
    count_breaks_impl::<Chunk>(&text.as_bytes()[..i])
}

/// Converts from line-index to byte-index in a string slice.
///
/// Returns the byte index of the start of the specified line.  Line 0 is
/// the start of the string, and subsequent lines start immediately
/// *after* each line break character.
///
/// Any past-the-end index will return the one-past-the-end byte index.
///
/// Runs in O(N) time.
#[inline]
pub fn to_byte_idx(text: &str, line_idx: usize) -> usize {
    to_byte_idx_impl::<Chunk>(text.as_bytes(), line_idx)
}

//-------------------------------------------------------------

#[inline(always)]
fn to_byte_idx_impl<T: ByteChunk>(text: &[u8], line_idx: usize) -> usize {
    // Get `middle` so we can do more efficient chunk-based counting.
    // We can't use this to get `end`, however, because the start index of
    // `end` actually depends on the accumulating line counts during the
    // counting process.
    let (start, middle, _) = unsafe { text.align_to::<T>() };

    let mut byte_count = 0;
    let mut lf_count = 0;

    // Take care of any unaligned bytes at the beginning.
    for byte in start.iter() {
        if lf_count == line_idx {
            break;
        }
        lf_count += (*byte == 0x0A) as usize;
        byte_count += 1;
    }

    // Process chunks in the fast path.
    let mut chunks = middle;
    let mut max_round_len = (line_idx - lf_count) / T::MAX_ACC;
    while max_round_len > 0 && !chunks.is_empty() {
        // Choose the largest number of chunks we can do this round
        // that will neither overflow `max_acc` nor blast past the
        // remaining line breaks we're looking for.
        let round_len = T::MAX_ACC.min(max_round_len).min(chunks.len());
        max_round_len -= round_len;
        let round = &chunks[..round_len];
        chunks = &chunks[round_len..];

        // Process the chunks in this round.
        let mut acc = T::zero();
        for chunk in round.iter() {
            acc = acc.add(chunk.cmp_eq_byte(0x0A));
        }
        lf_count += acc.sum_bytes();
        byte_count += T::SIZE * round_len;
    }

    // Process chunks in the slow path.
    for chunk in chunks.iter() {
        let new_lf_count = lf_count + chunk.cmp_eq_byte(0x0A).sum_bytes();
        if new_lf_count >= line_idx {
            break;
        }
        lf_count = new_lf_count;
        byte_count += T::SIZE;
    }

    // Take care of any unaligned bytes at the end.
    let end = &text[byte_count..];
    for byte in end.iter() {
        if lf_count == line_idx {
            break;
        }
        lf_count += (*byte == 0x0A) as usize;
        byte_count += 1;
    }

    // Finish up
    byte_count
}

/// Counts the line breaks in a utf8 encoded string.
///
/// The following unicode sequences are considered newlines by this function:
/// - u{000A}        (Line Feed)
#[inline(always)]
fn count_breaks_impl<T: ByteChunk>(text: &[u8]) -> usize {
    if text.len() < T::SIZE {
        // Bypass the more complex routine for short strings, where the
        // complexity hurts performance.
        text.iter().map(|byte| (*byte == 0x0A) as usize).sum()
    } else {
        // Get `middle` so we can do more efficient chunk-based counting.
        let (start, middle, end) = unsafe { text.align_to::<T>() };

        let mut count = 0;

        // Take care of unaligned bytes at the beginning.
        for byte in start.iter() {
            count += (*byte == 0x0A) as usize;
        }

        // Take care of the middle bytes in big chunks.
        for chunks in middle.chunks(T::MAX_ACC) {
            let mut acc = T::zero();
            for chunk in chunks.iter() {
                acc = acc.add(chunk.cmp_eq_byte(0x0A));
            }
            count += acc.sum_bytes();
        }

        // Take care of unaligned bytes at the end.
        for byte in end.iter() {
            count += (*byte == 0x0A) as usize;
        }

        count
    }
}

//=============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 124 bytes, 100 chars, 4 lines
    const TEXT_LINES: &str = "Hello there!  How're you doing?\nIt's \
                              a fine day, isn't it?\nAren't you glad \
                              we're alive?\nこんにちは、みんなさん！";

    #[test]
    fn count_breaks_01() {
        let text = "\nHello\u{000D}\nせ\u{000B}か\u{000C}い\u{0085}. \
                    There\nis something.\u{2029}";
        assert_eq!(45, text.len());
        assert_eq!(3, count_breaks(text));
    }

    #[test]
    fn from_byte_idx_01() {
        let text = "Here\nare\nsome\nwords";
        assert_eq!(0, from_byte_idx(text, 0));
        assert_eq!(0, from_byte_idx(text, 4));
        assert_eq!(1, from_byte_idx(text, 5));
        assert_eq!(1, from_byte_idx(text, 8));
        assert_eq!(2, from_byte_idx(text, 9));
        assert_eq!(2, from_byte_idx(text, 13));
        assert_eq!(3, from_byte_idx(text, 14));
        assert_eq!(3, from_byte_idx(text, 19));
    }

    #[test]
    fn from_byte_idx_02() {
        let text = "\nHere\nare\nsome\nwords\n";
        assert_eq!(0, from_byte_idx(text, 0));
        assert_eq!(1, from_byte_idx(text, 1));
        assert_eq!(1, from_byte_idx(text, 5));
        assert_eq!(2, from_byte_idx(text, 6));
        assert_eq!(2, from_byte_idx(text, 9));
        assert_eq!(3, from_byte_idx(text, 10));
        assert_eq!(3, from_byte_idx(text, 14));
        assert_eq!(4, from_byte_idx(text, 15));
        assert_eq!(4, from_byte_idx(text, 20));
        assert_eq!(5, from_byte_idx(text, 21));
    }

    #[test]
    fn from_byte_idx_03() {
        let text = "Here\r\nare\r\nsome\r\nwords";
        assert_eq!(0, from_byte_idx(text, 0));
        assert_eq!(0, from_byte_idx(text, 4));
        assert_eq!(0, from_byte_idx(text, 5));
        assert_eq!(1, from_byte_idx(text, 6));
        assert_eq!(1, from_byte_idx(text, 9));
        assert_eq!(1, from_byte_idx(text, 10));
        assert_eq!(2, from_byte_idx(text, 11));
        assert_eq!(2, from_byte_idx(text, 15));
        assert_eq!(2, from_byte_idx(text, 16));
        assert_eq!(3, from_byte_idx(text, 17));
    }

    #[test]
    fn from_byte_idx_04() {
        // Line 0
        for i in 0..32 {
            assert_eq!(0, from_byte_idx(TEXT_LINES, i));
        }

        // Line 1
        for i in 32..59 {
            assert_eq!(1, from_byte_idx(TEXT_LINES, i));
        }

        // Line 2
        for i in 59..88 {
            assert_eq!(2, from_byte_idx(TEXT_LINES, i));
        }

        // Line 3
        for i in 88..125 {
            assert_eq!(3, from_byte_idx(TEXT_LINES, i));
        }

        // Past the end
        for i in 125..130 {
            assert_eq!(3, from_byte_idx(TEXT_LINES, i));
        }
    }

    #[test]
    fn to_byte_idx_01() {
        let text = "Here\r\nare\r\nsome\r\nwords";
        assert_eq!(0, to_byte_idx(text, 0));
        assert_eq!(6, to_byte_idx(text, 1));
        assert_eq!(11, to_byte_idx(text, 2));
        assert_eq!(17, to_byte_idx(text, 3));
    }

    #[test]
    fn to_byte_idx_02() {
        let text = "\nHere\nare\nsome\nwords\n";
        assert_eq!(0, to_byte_idx(text, 0));
        assert_eq!(1, to_byte_idx(text, 1));
        assert_eq!(6, to_byte_idx(text, 2));
        assert_eq!(10, to_byte_idx(text, 3));
        assert_eq!(15, to_byte_idx(text, 4));
        assert_eq!(21, to_byte_idx(text, 5));
    }

    #[test]
    fn to_byte_idx_03() {
        assert_eq!(0, to_byte_idx(TEXT_LINES, 0));
        assert_eq!(32, to_byte_idx(TEXT_LINES, 1));
        assert_eq!(59, to_byte_idx(TEXT_LINES, 2));
        assert_eq!(88, to_byte_idx(TEXT_LINES, 3));

        // Past end
        assert_eq!(124, to_byte_idx(TEXT_LINES, 4));
        assert_eq!(124, to_byte_idx(TEXT_LINES, 5));
        assert_eq!(124, to_byte_idx(TEXT_LINES, 6));
    }

    #[test]
    fn line_byte_round_trip() {
        let text = "\nHere\nare\nsome\nwords\n";
        assert_eq!(6, to_byte_idx(text, from_byte_idx(text, 6)));
        assert_eq!(2, from_byte_idx(text, to_byte_idx(text, 2)));

        assert_eq!(0, to_byte_idx(text, from_byte_idx(text, 0)));
        assert_eq!(0, from_byte_idx(text, to_byte_idx(text, 0)));

        assert_eq!(21, to_byte_idx(text, from_byte_idx(text, 21)));
        assert_eq!(5, from_byte_idx(text, to_byte_idx(text, 5)));
    }
}
