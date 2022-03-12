//! Index by Unicode lines.
//!
//! This recognizes all line breaks defined in
//! [Unicode Annex #14](https://www.unicode.org/reports/tr14/):
//!
//! - `U+000A`          &mdash; LF (Line Feed)
//! - `U+000B`          &mdash; VT (Vertical Tab)
//! - `U+000C`          &mdash; FF (Form Feed)
//! - `U+000D`          &mdash; CR (Carriage Return)
//! - `U+0085`          &mdash; NEL (Next Line)
//! - `U+2028`          &mdash; Line Separator
//! - `U+2029`          &mdash; Paragraph Separator
//! - `U+000D` `U+000A` &mdash; CRLF (Carriage Return + Line Feed)

use crate::alignment_diff;
use crate::byte_chunk::{ByteChunk, Chunk};

/// Counts the line breaks in a string slice.
///
/// Runs in O(N) time.
#[inline]
pub fn count_breaks(text: &str) -> usize {
    count_breaks_internal::<Chunk>(text)
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
    let nl_count = count_breaks(&text[..byte_idx]);
    if is_not_crlf_middle(byte_idx, text.as_bytes()) {
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
    to_byte_idx_inner::<Chunk>(text, line_idx)
}

//-------------------------------------------------------------

#[inline]
fn is_not_crlf_middle(byte_idx: usize, text: &[u8]) -> bool {
    debug_assert!(byte_idx <= text.len());

    if byte_idx == 0 || byte_idx == text.len() {
        true
    } else {
        (text[byte_idx] >> 6 != 0b10) && ((text[byte_idx - 1] != 0x0D) | (text[byte_idx] != 0x0A))
    }
}

#[inline(always)]
fn to_byte_idx_inner<T: ByteChunk>(text: &str, line_idx: usize) -> usize {
    let mut bytes = text.as_bytes();
    let mut line_break_count = 0;

    // Handle unaligned bytes at the start.
    let aligned_idx = alignment_diff::<T>(bytes);
    if aligned_idx > 0 {
        let result = count_breaks_up_to(bytes, aligned_idx, line_idx);
        line_break_count += result.0;
        bytes = &bytes[result.1..];
    }

    // Count line breaks in big chunks.
    if alignment_diff::<T>(bytes) == 0 {
        while bytes.len() >= T::size() {
            // Unsafe because the called function depends on correct alignment.
            let tmp = unsafe { count_breaks_in_chunk_from_ptr::<T>(bytes) }.sum_bytes();
            if tmp + line_break_count >= line_idx {
                break;
            }
            line_break_count += tmp;

            bytes = &bytes[T::size()..];
        }
    }

    // Handle unaligned bytes at the end.
    let result = count_breaks_up_to(bytes, bytes.len(), line_idx - line_break_count);
    bytes = &bytes[result.1..];

    // Finish up
    let mut byte_idx = text.len() - bytes.len();
    while !text.is_char_boundary(byte_idx) {
        byte_idx += 1;
    }
    byte_idx
}

/// Uses bit-fiddling magic to count line breaks really quickly.
///
/// The following unicode sequences are considered newlines by this function:
/// - u{000A}        (Line Feed)
/// - u{000B}        (Vertical Tab)
/// - u{000C}        (Form Feed)
/// - u{000D}        (Carriage Return)
/// - u{000D}u{000A} (Carriage Return + Line Feed)
/// - u{0085}        (Next Line)
/// - u{2028}        (Line Separator)
/// - u{2029}        (Paragraph Separator)
#[inline(always)]
fn count_breaks_internal<T: ByteChunk>(text: &str) -> usize {
    let mut bytes = text.as_bytes();
    let mut count = 0;

    // Handle unaligned bytes at the start.
    let aligned_idx = alignment_diff::<T>(bytes);
    if aligned_idx > 0 {
        let result = count_breaks_up_to(bytes, aligned_idx, bytes.len());
        count += result.0;
        bytes = &bytes[result.1..];
    }

    // Count line breaks in big chunks.
    let mut i = 0;
    let mut acc = T::splat(0);
    while bytes.len() >= T::size() {
        // Unsafe because the called function depends on correct alignment.
        acc = acc.add(unsafe { count_breaks_in_chunk_from_ptr::<T>(bytes) });
        i += 1;
        if i == T::max_acc() {
            i = 0;
            count += acc.sum_bytes();
            acc = T::splat(0);
        }
        bytes = &bytes[T::size()..];
    }
    count += acc.sum_bytes();

    // Handle unaligned bytes at the end.
    count += count_breaks_up_to(bytes, bytes.len(), bytes.len()).0;

    count
}

/// Used internally in the line-break counting functions.
///
/// Counts line breaks a byte at a time up to a maximum number of bytes and
/// line breaks, and returns the counted lines and how many bytes were processed.
#[inline(always)]
#[allow(clippy::if_same_then_else)]
fn count_breaks_up_to(bytes: &[u8], max_bytes: usize, max_breaks: usize) -> (usize, usize) {
    let mut ptr = 0;
    let mut count = 0;
    while ptr < max_bytes && count < max_breaks {
        let byte = bytes[ptr];

        // Handle u{000A}, u{000B}, u{000C}, and u{000D}
        if (0x0A..=0x0D).contains(&byte) {
            count += 1;

            // Check for CRLF and and subtract 1 if it is,
            // since it will be caught in the next iteration
            // with the LF.
            if byte == 0x0D && (ptr + 1) < bytes.len() && bytes[ptr + 1] == 0x0A {
                count -= 1;
            }
        }
        // Handle u{0085}
        else if byte == 0xC2 && (ptr + 1) < bytes.len() && bytes[ptr + 1] == 0x85 {
            count += 1;
        }
        // Handle u{2028} and u{2029}
        else if byte == 0xE2
            && (ptr + 2) < bytes.len()
            && bytes[ptr + 1] == 0x80
            && (bytes[ptr + 2] >> 1) == 0x54
        {
            count += 1;
        }

        ptr += 1;
    }

    (count, ptr)
}

/// Used internally in the line-break counting functions.
///
/// The start of `bytes` MUST be aligned as type T, and `bytes` MUST be at
/// least as large (in bytes) as T.  If these invariants are not met, bad
/// things could potentially happen.  Hence why this function is unsafe.
#[inline(always)]
unsafe fn count_breaks_in_chunk_from_ptr<T: ByteChunk>(bytes: &[u8]) -> T {
    let c = {
        // The only unsafe bits of the function are in this block.
        debug_assert_eq!(bytes.align_to::<T>().0.len(), 0);
        debug_assert!(bytes.len() >= T::size());
        // This unsafe cast is for performance reasons: going through e.g.
        // `align_to()` results in a significant drop in performance.
        *(bytes.as_ptr() as *const T)
    };
    let end_i = T::size();

    let mut acc = T::splat(0);

    // Calculate the flags we're going to be working with.
    let nl_1_flags = c.cmp_eq_byte(0xC2);
    let sp_1_flags = c.cmp_eq_byte(0xE2);
    let all_flags = c.bytes_between_127(0x09, 0x0E);
    let cr_flags = c.cmp_eq_byte(0x0D);

    // Next Line: u{0085}
    if !nl_1_flags.is_zero() {
        let nl_2_flags = c.cmp_eq_byte(0x85).shift_back_lex(1);
        let flags = nl_1_flags.bitand(nl_2_flags);
        acc = acc.add(flags);

        // Handle ending boundary
        if bytes.len() > end_i && bytes[end_i - 1] == 0xC2 && bytes[end_i] == 0x85 {
            acc = acc.inc_nth_from_end_lex_byte(0);
        }
    }

    // Line Separator:      u{2028}
    // Paragraph Separator: u{2029}
    if !sp_1_flags.is_zero() {
        let sp_2_flags = c.cmp_eq_byte(0x80).shift_back_lex(1).bitand(sp_1_flags);
        if !sp_2_flags.is_zero() {
            let sp_3_flags = c
                .shr(1)
                .bitand(T::splat(!0x80))
                .cmp_eq_byte(0x54)
                .shift_back_lex(2);
            let sp_flags = sp_2_flags.bitand(sp_3_flags);
            acc = acc.add(sp_flags);
        }

        // Handle ending boundary
        if bytes.len() > end_i
            && bytes[end_i - 2] == 0xE2
            && bytes[end_i - 1] == 0x80
            && (bytes[end_i] >> 1) == 0x54
        {
            acc = acc.inc_nth_from_end_lex_byte(1);
        } else if bytes.len() > (end_i + 1)
            && bytes[end_i - 1] == 0xE2
            && bytes[end_i] == 0x80
            && (bytes[end_i + 1] >> 1) == 0x54
        {
            acc = acc.inc_nth_from_end_lex_byte(0);
        }
    }

    // Line Feed:                   u{000A}
    // Vertical Tab:                u{000B}
    // Form Feed:                   u{000C}
    // Carriage Return:             u{000D}
    // Carriage Return + Line Feed: u{000D}u{000A}
    acc = acc.add(all_flags);
    if !cr_flags.is_zero() {
        // Handle CRLF
        let lf_flags = c.cmp_eq_byte(0x0A);
        let crlf_flags = cr_flags.bitand(lf_flags.shift_back_lex(1));
        acc = acc.sub(crlf_flags);
        if bytes.len() > end_i && bytes[end_i - 1] == 0x0D && bytes[end_i] == 0x0A {
            acc = acc.dec_last_lex_byte();
        }
    }

    acc
}

//-------------------------------------------------------------

/// An iterator that yields the byte indices of line breaks in a string.
/// A line break in this case is the point immediately *after* a newline
/// character.
///
/// The following unicode sequences are considered newlines by this function:
/// - u{000A}        (Line Feed)
/// - u{000B}        (Vertical Tab)
/// - u{000C}        (Form Feed)
/// - u{000D}        (Carriage Return)
/// - u{000D}u{000A} (Carriage Return + Line Feed)
/// - u{0085}        (Next Line)
/// - u{2028}        (Line Separator)
/// - u{2029}        (Paragraph Separator)
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
            if (0x0A..=0x0D).contains(&byte) {
                if byte == 0x0D {
                    // We're basically "peeking" here.
                    if let Some(0x0A) = self.byte_itr.clone().next() {
                        self.byte_itr.next();
                        self.byte_idx += 1;
                    }
                }
                return Some(self.byte_idx);
            }
            // Handle u{0085}
            else if byte == 0xC2 {
                self.byte_idx += 1;
                if let Some(0x85) = self.byte_itr.next() {
                    return Some(self.byte_idx);
                }
            }
            // Handle u{2028} and u{2029}
            else if byte == 0xE2 {
                self.byte_idx += 2;
                let byte2 = self.byte_itr.next().unwrap();
                let byte3 = self.byte_itr.next().unwrap() >> 1;
                if byte2 == 0x80 && byte3 == 0x54 {
                    return Some(self.byte_idx);
                }
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
        let text = "\u{000A}Hello\u{000D}\u{000A}\u{000D}せ\u{000B}か\u{000C}い\u{0085}. \
                    There\u{2028}is something.\u{2029}";
        let mut itr = LineBreakIter::new(text);
        assert_eq!(48, text.len());
        assert_eq!(Some(1), itr.next());
        assert_eq!(Some(8), itr.next());
        assert_eq!(Some(9), itr.next());
        assert_eq!(Some(13), itr.next());
        assert_eq!(Some(17), itr.next());
        assert_eq!(Some(22), itr.next());
        assert_eq!(Some(32), itr.next());
        assert_eq!(Some(48), itr.next());
        assert_eq!(None, itr.next());
    }

    #[test]
    fn count_breaks_01() {
        let text = "\u{000A}Hello\u{000D}\u{000A}\u{000D}せ\u{000B}か\u{000C}い\u{0085}. \
                    There\u{2028}is something.\u{2029}";
        assert_eq!(48, text.len());
        assert_eq!(8, count_breaks(text));
    }

    #[test]
    fn count_breaks_02() {
        let text = "\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}";
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
