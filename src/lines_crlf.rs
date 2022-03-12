//! Index by lines (carriage return and line feed).
//!
//! This module recognizes the following as line breaks:
//!
//! - `U+000A`          &mdash; LF (Line Feed)
//! - `U+000D`          &mdash; CR (Carriage Return)
//! - `U+000D` `U+000A` &mdash; CRLF (Carriage Return + Line Feed)
//!
//! (Note: if you only want to recognize LF and CRLF, without
//! recognizing CR individually, see the [`lines_lf`](crate::lines_lf) module.)

use crate::byte_chunk::{ByteChunk, Chunk};

/// Counts the line breaks in a string slice.
///
/// Runs in O(N) time.
#[inline]
pub fn count_breaks(text: &str) -> usize {
    count_breaks_internal::<Chunk>(text.as_bytes())
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
    let mut byte_idx = byte_idx.min(text.len());
    while !text.is_char_boundary(byte_idx) {
        byte_idx -= 1;
    }
    let nl_count = count_breaks_internal::<Chunk>(&text.as_bytes()[..byte_idx]);
    if crate::is_not_crlf_middle(byte_idx, text.as_bytes()) {
        nl_count
    } else {
        nl_count - 1
    }
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
    to_byte_idx_inner::<Chunk>(text.as_bytes(), line_idx)
}

//-------------------------------------------------------------

#[inline(always)]
fn to_byte_idx_inner<T: ByteChunk>(text: &[u8], line_idx: usize) -> usize {
    // Get `middle` so we can do more efficient chunk-based counting.
    // We can't use this to get `end`, however, because the start index of
    // `end` actually depends on the accumulating line counts during the
    // counting process.
    let (start, middle, _) = unsafe { text.align_to::<T>() };

    let mut byte_count = 0;
    let mut break_count = 0;

    // Take care of any unaligned bytes at the beginning.
    while byte_count < start.len() && break_count < line_idx {
        if text[byte_count] == 0x0A {
            break_count += 1;
        } else if text[byte_count] == 0x0D && text.get(byte_count + 1) != Some(&0x0A) {
            break_count += 1;
        }
        byte_count += 1;
    }

    // Use chunks to count multiple bytes at once.
    let mut chunk_i = 0;
    let mut acc = T::zero();
    let mut acc_i = 0;
    let mut boundary_crlf_count = 0;
    while chunk_i < middle.len()
        && (break_count + (T::size() * (acc_i + 1)) - boundary_crlf_count) < line_idx
    {
        let lf_flags = middle[chunk_i].cmp_eq_byte(0x0A);
        let cr_flags = middle[chunk_i].cmp_eq_byte(0x0D);
        let crlf_flags = cr_flags.bitand(lf_flags.shift_back_lex(1));

        acc = acc.add(lf_flags).add(cr_flags.sub(crlf_flags));
        acc_i += 1;
        if acc_i >= T::max_acc()
            || (break_count + (T::size() * (acc_i + 1)) - boundary_crlf_count) >= line_idx
        {
            break_count += acc.sum_bytes();
            acc_i = 0;
            acc = T::zero();
        }
        chunk_i += 1;
        byte_count += T::size();

        // Handle potential boundary CRLF.
        boundary_crlf_count +=
            (text[byte_count - 1] == 0x0D && text.get(byte_count) == Some(&0x0A)) as usize;
    }
    break_count += acc.sum_bytes();
    break_count -= boundary_crlf_count;

    // Take care of any unaligned bytes at the end.
    while byte_count < text.len() && break_count < line_idx {
        if text[byte_count] == 0x0A {
            break_count += 1;
        } else if text[byte_count] == 0x0D && text.get(byte_count + 1) != Some(&0x0A) {
            break_count += 1;
        }
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
fn count_breaks_internal<T: ByteChunk>(text: &[u8]) -> usize {
    // Get `middle` so we can do more efficient chunk-based counting.
    let (start, middle, end) = unsafe { text.align_to::<T>() };

    let mut text_i = 0;

    let mut count = 0;
    let mut last_was_cr = false;

    // Take care of unaligned bytes at the beginning.
    for byte in start.iter().copied() {
        let is_lf = byte == 0x0A;
        let is_cr = byte == 0x0D;
        count += (is_cr | (is_lf & !last_was_cr)) as usize;
        last_was_cr = is_cr;
    }
    text_i += start.len();

    // Take care of the middle bytes in big chunks.
    let mut i = 0;
    let mut acc = T::zero();
    for chunk in middle.iter() {
        let lf_flags = chunk.cmp_eq_byte(0x0A);
        let cr_flags = chunk.cmp_eq_byte(0x0D);
        let crlf_flags = cr_flags.bitand(lf_flags.shift_back_lex(1));

        acc = acc.add(lf_flags).add(cr_flags.sub(crlf_flags));
        i += 1;
        if i >= T::max_acc() {
            i = 0;
            count += acc.sum_bytes();
            acc = T::zero();
        }

        // Handle potential CRLF across boundaries.
        count -= ((text[text_i] == 0x0A) & last_was_cr) as usize;
        text_i += T::size();
        last_was_cr = text[text_i - 1] == 0x0D;
    }
    count += acc.sum_bytes();

    // Take care of unaligned bytes at the end.
    for byte in end.iter().copied() {
        let is_lf = byte == 0x0A;
        let is_cr = byte == 0x0D;
        count += (is_cr | (is_lf & !last_was_cr)) as usize;
        last_was_cr = is_cr;
    }

    count
}

//-------------------------------------------------------------

/// An iterator that yields the byte indices of line breaks in a string.
/// A line break in this case is the point immediately *after* a newline
/// character.
///
/// The following unicode sequences are considered newlines by this function:
/// - u{000A}        (Line Feed)
#[allow(unused)] // Used in tests, as reference solution.
struct LineBreakIter<'a> {
    byte_itr: core::str::Bytes<'a>,
    byte_idx: usize,
}

#[allow(unused)]
impl<'a> LineBreakIter<'a> {
    #[inline]
    fn new(text: &str) -> LineBreakIter {
        LineBreakIter {
            byte_itr: text.bytes(),
            byte_idx: 0,
        }
    }
}

impl<'a> Iterator for LineBreakIter<'a> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        while let Some(byte) = self.byte_itr.next() {
            self.byte_idx += 1;
            // Handle u{000A}, u{000B}, u{000C}, and u{000D}
            if byte == 0x0A {
                return Some(self.byte_idx);
            } else if byte == 0x0D {
                // Peeking ahead to check for CRLF.
                if let Some(0x0A) = self.byte_itr.clone().next() {
                    self.byte_itr.next();
                    self.byte_idx += 1;
                }
                return Some(self.byte_idx);
            }
        }

        return None;
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
    fn line_breaks_iter_01() {
        let text = "\u{000A}Hello\u{000D}\u{000A}せ\u{000D}か\u{000D}い\u{0085}. \
                    There\u{000A}is something.\u{2029}";
        let mut itr = LineBreakIter::new(text);
        assert_eq!(45, text.len());
        assert_eq!(Some(1), itr.next());
        assert_eq!(Some(8), itr.next());
        assert_eq!(Some(12), itr.next());
        assert_eq!(Some(16), itr.next());
        assert_eq!(Some(29), itr.next());
        assert_eq!(None, itr.next());
    }

    #[test]
    fn count_breaks_01() {
        let text = "\u{000A}Hello\u{000D}\u{000A}せ\u{000B}か\u{000C}い\u{0085}. \
                    There\u{000A}is something.\u{2029}";
        assert_eq!(45, text.len());
        assert_eq!(3, count_breaks(text));
    }

    #[test]
    fn count_breaks_02() {
        let text = "\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}";
        assert_eq!(count_breaks(text), LineBreakIter::new(text).count());
    }

    #[test]
    fn count_breaks_03() {
        let bytes = &[0x0Au8; 723];
        let text = core::str::from_utf8(bytes).unwrap();
        assert_eq!(count_breaks(text), LineBreakIter::new(text).count());
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
