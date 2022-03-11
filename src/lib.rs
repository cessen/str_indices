#![no_std]

//! Utility functions for utf8 string slices.

mod byte_chunk;
use byte_chunk::ByteChunk;

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64;

/// Converts from byte-index to char-index in a string slice.
///
/// If the byte is in the middle of a multi-byte char, returns the index of
/// the char that the byte belongs to.
///
/// Any past-the-end index will return the one-past-the-end char index.
///
/// Runs in O(N) time.
#[inline]
pub fn byte_to_char_idx(text: &str, byte_idx: usize) -> usize {
    let count = count_chars_in_bytes(&text.as_bytes()[0..(byte_idx + 1).min(text.len())]);
    if byte_idx < text.len() {
        count - 1
    } else {
        count
    }
}

/// Converts from byte-index to line-index in a string slice.
///
/// This is equivalent to counting the line endings before the given byte.
///
/// Any past-the-end index will return the last line index.
///
/// Runs in O(N) time.
#[inline]
pub fn byte_to_line_idx(text: &str, byte_idx: usize) -> usize {
    let mut byte_idx = byte_idx.min(text.len());
    while !text.is_char_boundary(byte_idx) {
        byte_idx -= 1;
    }
    let nl_count = count_line_breaks(&text[..byte_idx]);
    if is_not_crlf_middle(byte_idx, text.as_bytes()) {
        nl_count
    } else {
        nl_count - 1
    }
}

#[inline]
fn is_not_crlf_middle(byte_idx: usize, text: &[u8]) -> bool {
    debug_assert!(byte_idx <= text.len());

    if byte_idx == 0 || byte_idx == text.len() {
        true
    } else {
        (text[byte_idx] >> 6 != 0b10) && ((text[byte_idx - 1] != 0x0D) | (text[byte_idx] != 0x0A))
    }
}

/// Converts from char-index to byte-index in a string slice.
///
/// Any past-the-end index will return the one-past-the-end byte index.
///
/// Runs in O(N) time.
#[inline]
pub fn char_to_byte_idx(text: &str, char_idx: usize) -> usize {
    #[cfg(target_arch = "x86_64")]
    return char_to_byte_idx_inner::<x86_64::__m128i>(text, char_idx);

    // Fallback for other platforms.
    #[cfg(not(any(target_arch = "x86_64")))]
    char_to_byte_idx_inner::<usize>(text, char_idx)
}

#[inline(always)]
fn char_to_byte_idx_inner<T: ByteChunk>(text: &str, char_idx: usize) -> usize {
    // Get `middle` so we can do more efficient chunk-based counting.
    // We can't use this to get `end`, however, because the start index of
    // `end` actually depends on the accumulating char counts during the
    // counting process.
    let (start, middle, _) = unsafe { text.as_bytes().align_to::<T>() };

    let mut byte_count = 0;
    let mut char_count = 0;

    // Take care of any unaligned bytes at the beginning.
    let mut i = 0;
    while i < start.len() && char_count <= char_idx {
        char_count += ((start[i] & 0xC0) != 0x80) as usize;
        i += 1;
    }
    byte_count += i;

    // Use chunks to count multiple bytes at once, using bit-fiddling magic.
    let mut i = 0;
    let mut acc = T::splat(0);
    let mut acc_i = 0;
    while i < middle.len() && (char_count + (T::size() * (acc_i + 1))) <= char_idx {
        acc = acc.add(middle[i].bitand(T::splat(0xc0)).cmp_eq_byte(0x80));
        acc_i += 1;
        if acc_i == T::max_acc() || (char_count + (T::size() * (acc_i + 1))) >= char_idx {
            char_count += (T::size() * acc_i) - acc.sum_bytes();
            acc_i = 0;
            acc = T::splat(0);
        }
        i += 1;
    }
    char_count += (T::size() * acc_i) - acc.sum_bytes();
    byte_count += i * T::size();

    // Take care of any unaligned bytes at the end.
    let end = &text.as_bytes()[byte_count..];
    let mut i = 0;
    while i < end.len() && char_count <= char_idx {
        char_count += ((end[i] & 0xC0) != 0x80) as usize;
        i += 1;
    }
    byte_count += i;

    // Finish up
    if byte_count == text.len() && char_count <= char_idx {
        byte_count
    } else {
        byte_count - 1
    }
}

/// Converts from char-index to line-index in a string slice.
///
/// This is equivalent to counting the line endings before the given char.
///
/// Any past-the-end index will return the last line index.
///
/// Runs in O(N) time.
#[inline]
pub fn char_to_line_idx(text: &str, char_idx: usize) -> usize {
    byte_to_line_idx(text, char_to_byte_idx(text, char_idx))
}

/// Converts from line-index to byte-index in a string slice.
///
/// More specifically, this returns the index of the first byte of the given
/// line.
///
/// Any past-the-end index will return the one-past-the-end byte index.
///
/// Runs in O(N) time.
#[inline]
pub fn line_to_byte_idx(text: &str, line_idx: usize) -> usize {
    #[cfg(target_arch = "x86_64")]
    return line_to_byte_idx_inner::<x86_64::__m128i>(text, line_idx);

    // Fallback for other platforms.
    #[cfg(not(any(target_arch = "x86_64")))]
    line_to_byte_idx_inner::<usize>(text, line_idx)
}

#[inline(always)]
fn line_to_byte_idx_inner<T: ByteChunk>(text: &str, line_idx: usize) -> usize {
    let mut bytes = text.as_bytes();
    let mut line_break_count = 0;

    // Handle unaligned bytes at the start.
    let aligned_idx = alignment_diff::<T>(bytes);
    if aligned_idx > 0 {
        let result = count_line_breaks_up_to(bytes, aligned_idx, line_idx);
        line_break_count += result.0;
        bytes = &bytes[result.1..];
    }

    // Count line breaks in big chunks.
    if alignment_diff::<T>(bytes) == 0 {
        while bytes.len() >= T::size() {
            // Unsafe because the called function depends on correct alignment.
            let tmp = unsafe { count_line_breaks_in_chunk_from_ptr::<T>(bytes) }.sum_bytes();
            if tmp + line_break_count >= line_idx {
                break;
            }
            line_break_count += tmp;

            bytes = &bytes[T::size()..];
        }
    }

    // Handle unaligned bytes at the end.
    let result = count_line_breaks_up_to(bytes, bytes.len(), line_idx - line_break_count);
    bytes = &bytes[result.1..];

    // Finish up
    let mut byte_idx = text.len() - bytes.len();
    while !text.is_char_boundary(byte_idx) {
        byte_idx += 1;
    }
    byte_idx
}

/// Converts from line-index to char-index in a string slice.
///
/// More specifically, this returns the index of the first char of the given
/// line.
///
/// Any past-the-end index will return the one-past-the-end char index.
///
/// Runs in O(N) time.
#[inline]
pub fn line_to_char_idx(text: &str, line_idx: usize) -> usize {
    byte_to_char_idx(text, line_to_byte_idx(text, line_idx))
}

/// Counts the utf16 surrogate pairs that would be in `text` if it were encoded
/// as utf16.
#[inline]
fn count_utf16_surrogates(text: &str) -> usize {
    count_utf16_surrogates_in_bytes(text.as_bytes())
}

#[inline]
fn count_utf16_surrogates_in_bytes(text: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    return count_utf16_surrogates_internal::<x86_64::__m128i>(text);

    // Fallback for other platforms.
    #[cfg(not(any(target_arch = "x86_64")))]
    count_utf16_surrogates_internal::<usize>(text)
}

#[inline(always)]
fn count_utf16_surrogates_internal<T: ByteChunk>(text: &[u8]) -> usize {
    // Get `middle` for more efficient chunk-based counting.
    let (start, middle, end) = unsafe { text.align_to::<T>() };

    let mut utf16_surrogate_count = 0;

    // Take care of unaligned bytes at the beginning.
    for byte in start.iter() {
        utf16_surrogate_count += ((byte & 0xf0) == 0xf0) as usize;
    }

    // Take care of the middle bytes in big chunks.
    let mut i = 0;
    let mut acc = T::splat(0);
    for chunk in middle.iter() {
        let tmp = chunk.bitand(T::splat(0xf0)).cmp_eq_byte(0xf0);
        acc = acc.add(tmp);
        i += 1;
        if i == T::max_acc() {
            i = 0;
            utf16_surrogate_count += acc.sum_bytes();
            acc = T::splat(0);
        }
    }
    utf16_surrogate_count += acc.sum_bytes();

    // Take care of unaligned bytes at the end.
    for byte in end.iter() {
        utf16_surrogate_count += ((byte & 0xf0) == 0xf0) as usize;
    }

    utf16_surrogate_count
}

#[inline(always)]
pub fn byte_to_utf16_surrogate_idx(text: &str, byte_idx: usize) -> usize {
    count_utf16_surrogates(&text[..byte_idx])
}

#[inline(always)]
pub fn utf16_code_unit_to_char_idx(text: &str, utf16_idx: usize) -> usize {
    // TODO: optimized version.  This is pretty slow.  It isn't expected to be
    // used in performance critical functionality, so this isn't urgent.  But
    // might as well make it faster when we get the chance.
    let mut char_i = 0;
    let mut utf16_i = 0;
    for c in text.chars() {
        if utf16_idx <= utf16_i {
            break;
        }
        char_i += 1;
        utf16_i += c.len_utf16();
    }

    if utf16_idx < utf16_i {
        char_i -= 1;
    }

    char_i
}

//===========================================================================
// Internal
//===========================================================================

#[inline]
fn count_chars_in_bytes(text: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    return count_chars_internal::<x86_64::__m128i>(text);

    // Fallback for other platforms.
    #[cfg(not(any(target_arch = "x86_64")))]
    count_chars_internal::<usize>(text)
}

#[inline(always)]
fn count_chars_internal<T: ByteChunk>(text: &[u8]) -> usize {
    // Get `middle` for more efficient chunk-based counting.
    let (start, middle, end) = unsafe { text.align_to::<T>() };

    let mut inv_count = 0;

    // Take care of unaligned bytes at the beginning.
    for byte in start.iter() {
        inv_count += ((byte & 0xC0) == 0x80) as usize;
    }

    // Take care of the middle bytes in big chunks.
    let mut i = 0;
    let mut acc = T::splat(0);
    for chunk in middle.iter() {
        let tmp = chunk.bitand(T::splat(0xc0)).cmp_eq_byte(0x80);
        acc = acc.add(tmp);
        i += 1;
        if i == T::max_acc() {
            i = 0;
            inv_count += acc.sum_bytes();
            acc = T::splat(0);
        }
    }
    inv_count += acc.sum_bytes();

    // Take care of unaligned bytes at the end.
    for byte in end.iter() {
        inv_count += ((byte & 0xC0) == 0x80) as usize;
    }

    text.len() - inv_count
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
#[inline]
fn count_line_breaks(text: &str) -> usize {
    #[cfg(target_arch = "x86_64")]
    return count_line_breaks_internal::<x86_64::__m128i>(text);

    // Fallback for other platforms.
    #[cfg(not(any(target_arch = "x86_64")))]
    count_line_breaks_internal::<usize>(text)
}

#[inline(always)]
fn count_line_breaks_internal<T: ByteChunk>(text: &str) -> usize {
    let mut bytes = text.as_bytes();
    let mut count = 0;

    // Handle unaligned bytes at the start.
    let aligned_idx = alignment_diff::<T>(bytes);
    if aligned_idx > 0 {
        let result = count_line_breaks_up_to(bytes, aligned_idx, bytes.len());
        count += result.0;
        bytes = &bytes[result.1..];
    }

    // Count line breaks in big chunks.
    let mut i = 0;
    let mut acc = T::splat(0);
    while bytes.len() >= T::size() {
        // Unsafe because the called function depends on correct alignment.
        acc = acc.add(unsafe { count_line_breaks_in_chunk_from_ptr::<T>(bytes) });
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
    count += count_line_breaks_up_to(bytes, bytes.len(), bytes.len()).0;

    count
}

/// Used internally in the line-break counting functions.
///
/// Counts line breaks a byte at a time up to a maximum number of bytes and
/// line breaks, and returns the counted lines and how many bytes were processed.
#[inline(always)]
#[allow(clippy::if_same_then_else)]
fn count_line_breaks_up_to(bytes: &[u8], max_bytes: usize, max_breaks: usize) -> (usize, usize) {
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
unsafe fn count_line_breaks_in_chunk_from_ptr<T: ByteChunk>(bytes: &[u8]) -> T {
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

/// Returns the alignment difference between the start of `bytes` and the
/// type `T`.
///
/// Or put differently: returns how many bytes into `bytes` you need to walk
/// to reach the alignment of `T` in memory.
///
/// Will return 0 if already aligned at the start, and will return the length
/// of `bytes` if alignment is beyond the end of `bytes`.
#[inline(always)]
fn alignment_diff<T>(bytes: &[u8]) -> usize {
    let alignment = core::mem::align_of::<T>();
    let ptr = bytes.as_ptr() as usize;
    (alignment - ((ptr - 1) & (alignment - 1)) - 1).min(bytes.len())
}

//======================================================================

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

//======================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 124 bytes, 100 chars, 4 lines
    const TEXT_LINES: &str = "Hello there!  How're you doing?\nIt's \
                              a fine day, isn't it?\nAren't you glad \
                              we're alive?\nこんにちは、みんなさん！";

    #[test]
    fn count_chars_01() {
        let text = "Hello せかい! Hello せかい! Hello せかい! Hello せかい! Hello せかい!";

        assert_eq!(54, byte_to_char_idx(text, text.len()));
    }

    #[test]
    fn count_chars_02() {
        assert_eq!(100, byte_to_char_idx(TEXT_LINES, TEXT_LINES.len()));
    }

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
    fn count_line_breaks_01() {
        let text = "\u{000A}Hello\u{000D}\u{000A}\u{000D}せ\u{000B}か\u{000C}い\u{0085}. \
                    There\u{2028}is something.\u{2029}";
        assert_eq!(48, text.len());
        assert_eq!(8, count_line_breaks(text));
    }

    #[test]
    fn count_line_breaks_02() {
        let text = "\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}\u{000A}Hello world!  This is a longer text.\u{000D}\u{000A}\u{000D}To better test that skipping by usize doesn't mess things up.\u{000B}Hello せかい!\u{000C}\u{0085}Yet more text.  How boring.\u{2028}Hi.\u{2029}";
        assert_eq!(count_line_breaks(text), LineBreakIter::new(text).count());
    }

    #[test]
    fn byte_to_char_idx_01() {
        let text = "Hello せかい!";
        assert_eq!(0, byte_to_char_idx(text, 0));
        assert_eq!(1, byte_to_char_idx(text, 1));
        assert_eq!(6, byte_to_char_idx(text, 6));
        assert_eq!(6, byte_to_char_idx(text, 7));
        assert_eq!(6, byte_to_char_idx(text, 8));
        assert_eq!(7, byte_to_char_idx(text, 9));
        assert_eq!(7, byte_to_char_idx(text, 10));
        assert_eq!(7, byte_to_char_idx(text, 11));
        assert_eq!(8, byte_to_char_idx(text, 12));
        assert_eq!(8, byte_to_char_idx(text, 13));
        assert_eq!(8, byte_to_char_idx(text, 14));
        assert_eq!(9, byte_to_char_idx(text, 15));
        assert_eq!(10, byte_to_char_idx(text, 16));
        assert_eq!(10, byte_to_char_idx(text, 17));
        assert_eq!(10, byte_to_char_idx(text, 18));
        assert_eq!(10, byte_to_char_idx(text, 19));
    }

    #[test]
    fn byte_to_char_idx_02() {
        let text = "";
        assert_eq!(0, byte_to_char_idx(text, 0));
        assert_eq!(0, byte_to_char_idx(text, 1));

        let text = "h";
        assert_eq!(0, byte_to_char_idx(text, 0));
        assert_eq!(1, byte_to_char_idx(text, 1));
        assert_eq!(1, byte_to_char_idx(text, 2));

        let text = "hi";
        assert_eq!(0, byte_to_char_idx(text, 0));
        assert_eq!(1, byte_to_char_idx(text, 1));
        assert_eq!(2, byte_to_char_idx(text, 2));
        assert_eq!(2, byte_to_char_idx(text, 3));
    }

    #[test]
    fn byte_to_char_idx_03() {
        let text = "せかい";
        assert_eq!(0, byte_to_char_idx(text, 0));
        assert_eq!(0, byte_to_char_idx(text, 1));
        assert_eq!(0, byte_to_char_idx(text, 2));
        assert_eq!(1, byte_to_char_idx(text, 3));
        assert_eq!(1, byte_to_char_idx(text, 4));
        assert_eq!(1, byte_to_char_idx(text, 5));
        assert_eq!(2, byte_to_char_idx(text, 6));
        assert_eq!(2, byte_to_char_idx(text, 7));
        assert_eq!(2, byte_to_char_idx(text, 8));
        assert_eq!(3, byte_to_char_idx(text, 9));
        assert_eq!(3, byte_to_char_idx(text, 10));
        assert_eq!(3, byte_to_char_idx(text, 11));
        assert_eq!(3, byte_to_char_idx(text, 12));
    }

    #[test]
    fn byte_to_char_idx_04() {
        // Ascii range
        for i in 0..88 {
            assert_eq!(i, byte_to_char_idx(TEXT_LINES, i));
        }

        // Hiragana characters
        for i in 88..125 {
            assert_eq!(88 + ((i - 88) / 3), byte_to_char_idx(TEXT_LINES, i));
        }

        // Past the end
        for i in 125..130 {
            assert_eq!(100, byte_to_char_idx(TEXT_LINES, i));
        }
    }

    #[test]
    fn byte_to_line_idx_01() {
        let text = "Here\nare\nsome\nwords";
        assert_eq!(0, byte_to_line_idx(text, 0));
        assert_eq!(0, byte_to_line_idx(text, 4));
        assert_eq!(1, byte_to_line_idx(text, 5));
        assert_eq!(1, byte_to_line_idx(text, 8));
        assert_eq!(2, byte_to_line_idx(text, 9));
        assert_eq!(2, byte_to_line_idx(text, 13));
        assert_eq!(3, byte_to_line_idx(text, 14));
        assert_eq!(3, byte_to_line_idx(text, 19));
    }

    #[test]
    fn byte_to_line_idx_02() {
        let text = "\nHere\nare\nsome\nwords\n";
        assert_eq!(0, byte_to_line_idx(text, 0));
        assert_eq!(1, byte_to_line_idx(text, 1));
        assert_eq!(1, byte_to_line_idx(text, 5));
        assert_eq!(2, byte_to_line_idx(text, 6));
        assert_eq!(2, byte_to_line_idx(text, 9));
        assert_eq!(3, byte_to_line_idx(text, 10));
        assert_eq!(3, byte_to_line_idx(text, 14));
        assert_eq!(4, byte_to_line_idx(text, 15));
        assert_eq!(4, byte_to_line_idx(text, 20));
        assert_eq!(5, byte_to_line_idx(text, 21));
    }

    #[test]
    fn byte_to_line_idx_03() {
        let text = "Here\r\nare\r\nsome\r\nwords";
        assert_eq!(0, byte_to_line_idx(text, 0));
        assert_eq!(0, byte_to_line_idx(text, 4));
        assert_eq!(0, byte_to_line_idx(text, 5));
        assert_eq!(1, byte_to_line_idx(text, 6));
        assert_eq!(1, byte_to_line_idx(text, 9));
        assert_eq!(1, byte_to_line_idx(text, 10));
        assert_eq!(2, byte_to_line_idx(text, 11));
        assert_eq!(2, byte_to_line_idx(text, 15));
        assert_eq!(2, byte_to_line_idx(text, 16));
        assert_eq!(3, byte_to_line_idx(text, 17));
    }

    #[test]
    fn byte_to_line_idx_04() {
        // Line 0
        for i in 0..32 {
            assert_eq!(0, byte_to_line_idx(TEXT_LINES, i));
        }

        // Line 1
        for i in 32..59 {
            assert_eq!(1, byte_to_line_idx(TEXT_LINES, i));
        }

        // Line 2
        for i in 59..88 {
            assert_eq!(2, byte_to_line_idx(TEXT_LINES, i));
        }

        // Line 3
        for i in 88..125 {
            assert_eq!(3, byte_to_line_idx(TEXT_LINES, i));
        }

        // Past the end
        for i in 125..130 {
            assert_eq!(3, byte_to_line_idx(TEXT_LINES, i));
        }
    }

    #[test]
    fn char_to_byte_idx_01() {
        let text = "Hello せかい!";
        assert_eq!(0, char_to_byte_idx(text, 0));
        assert_eq!(1, char_to_byte_idx(text, 1));
        assert_eq!(2, char_to_byte_idx(text, 2));
        assert_eq!(5, char_to_byte_idx(text, 5));
        assert_eq!(6, char_to_byte_idx(text, 6));
        assert_eq!(12, char_to_byte_idx(text, 8));
        assert_eq!(15, char_to_byte_idx(text, 9));
        assert_eq!(16, char_to_byte_idx(text, 10));
    }

    #[test]
    fn char_to_byte_idx_02() {
        let text = "せかい";
        assert_eq!(0, char_to_byte_idx(text, 0));
        assert_eq!(3, char_to_byte_idx(text, 1));
        assert_eq!(6, char_to_byte_idx(text, 2));
        assert_eq!(9, char_to_byte_idx(text, 3));
    }

    #[test]
    fn char_to_byte_idx_03() {
        let text = "Hello world!";
        assert_eq!(0, char_to_byte_idx(text, 0));
        assert_eq!(1, char_to_byte_idx(text, 1));
        assert_eq!(8, char_to_byte_idx(text, 8));
        assert_eq!(11, char_to_byte_idx(text, 11));
        assert_eq!(12, char_to_byte_idx(text, 12));
    }

    #[test]
    fn char_to_byte_idx_04() {
        let text = "Hello world! Hello せかい! Hello world! Hello せかい! \
                    Hello world! Hello せかい! Hello world! Hello せかい! \
                    Hello world! Hello せかい! Hello world! Hello せかい! \
                    Hello world! Hello せかい! Hello world! Hello せかい!";
        assert_eq!(0, char_to_byte_idx(text, 0));
        assert_eq!(30, char_to_byte_idx(text, 24));
        assert_eq!(60, char_to_byte_idx(text, 48));
        assert_eq!(90, char_to_byte_idx(text, 72));
        assert_eq!(115, char_to_byte_idx(text, 93));
        assert_eq!(120, char_to_byte_idx(text, 96));
        assert_eq!(150, char_to_byte_idx(text, 120));
        assert_eq!(180, char_to_byte_idx(text, 144));
        assert_eq!(210, char_to_byte_idx(text, 168));
        assert_eq!(239, char_to_byte_idx(text, 191));
    }

    #[test]
    fn char_to_byte_idx_05() {
        // Ascii range
        for i in 0..88 {
            assert_eq!(i, char_to_byte_idx(TEXT_LINES, i));
        }

        // Hiragana characters
        for i in 88..100 {
            assert_eq!(88 + ((i - 88) * 3), char_to_byte_idx(TEXT_LINES, i));
        }

        // Past the end
        for i in 100..110 {
            assert_eq!(124, char_to_byte_idx(TEXT_LINES, i));
        }
    }

    #[test]
    fn char_to_line_idx_01() {
        let text = "Hello せ\nか\nい!";
        assert_eq!(0, char_to_line_idx(text, 0));
        assert_eq!(0, char_to_line_idx(text, 7));
        assert_eq!(1, char_to_line_idx(text, 8));
        assert_eq!(1, char_to_line_idx(text, 9));
        assert_eq!(2, char_to_line_idx(text, 10));
    }

    #[test]
    fn char_to_line_idx_02() {
        // Line 0
        for i in 0..32 {
            assert_eq!(0, char_to_line_idx(TEXT_LINES, i));
        }

        // Line 1
        for i in 32..59 {
            assert_eq!(1, char_to_line_idx(TEXT_LINES, i));
        }

        // Line 2
        for i in 59..88 {
            assert_eq!(2, char_to_line_idx(TEXT_LINES, i));
        }

        // Line 3
        for i in 88..100 {
            assert_eq!(3, char_to_line_idx(TEXT_LINES, i));
        }

        // Past the end
        for i in 100..110 {
            assert_eq!(3, char_to_line_idx(TEXT_LINES, i));
        }
    }

    #[test]
    fn line_to_byte_idx_01() {
        let text = "Here\r\nare\r\nsome\r\nwords";
        assert_eq!(0, line_to_byte_idx(text, 0));
        assert_eq!(6, line_to_byte_idx(text, 1));
        assert_eq!(11, line_to_byte_idx(text, 2));
        assert_eq!(17, line_to_byte_idx(text, 3));
    }

    #[test]
    fn line_to_byte_idx_02() {
        let text = "\nHere\nare\nsome\nwords\n";
        assert_eq!(0, line_to_byte_idx(text, 0));
        assert_eq!(1, line_to_byte_idx(text, 1));
        assert_eq!(6, line_to_byte_idx(text, 2));
        assert_eq!(10, line_to_byte_idx(text, 3));
        assert_eq!(15, line_to_byte_idx(text, 4));
        assert_eq!(21, line_to_byte_idx(text, 5));
    }

    #[test]
    fn line_to_byte_idx_03() {
        assert_eq!(0, line_to_byte_idx(TEXT_LINES, 0));
        assert_eq!(32, line_to_byte_idx(TEXT_LINES, 1));
        assert_eq!(59, line_to_byte_idx(TEXT_LINES, 2));
        assert_eq!(88, line_to_byte_idx(TEXT_LINES, 3));

        // Past end
        assert_eq!(124, line_to_byte_idx(TEXT_LINES, 4));
        assert_eq!(124, line_to_byte_idx(TEXT_LINES, 5));
        assert_eq!(124, line_to_byte_idx(TEXT_LINES, 6));
    }

    #[test]
    fn line_to_char_idx_01() {
        let text = "Hello せ\nか\nい!";
        assert_eq!(0, line_to_char_idx(text, 0));
        assert_eq!(8, line_to_char_idx(text, 1));
        assert_eq!(10, line_to_char_idx(text, 2));
    }

    #[test]
    fn line_to_char_idx_02() {
        assert_eq!(0, line_to_char_idx(TEXT_LINES, 0));
        assert_eq!(32, line_to_char_idx(TEXT_LINES, 1));
        assert_eq!(59, line_to_char_idx(TEXT_LINES, 2));
        assert_eq!(88, line_to_char_idx(TEXT_LINES, 3));

        // Past end
        assert_eq!(100, line_to_char_idx(TEXT_LINES, 4));
        assert_eq!(100, line_to_char_idx(TEXT_LINES, 5));
        assert_eq!(100, line_to_char_idx(TEXT_LINES, 6));
    }

    #[test]
    fn line_byte_round_trip() {
        let text = "\nHere\nare\nsome\nwords\n";
        assert_eq!(6, line_to_byte_idx(text, byte_to_line_idx(text, 6)));
        assert_eq!(2, byte_to_line_idx(text, line_to_byte_idx(text, 2)));

        assert_eq!(0, line_to_byte_idx(text, byte_to_line_idx(text, 0)));
        assert_eq!(0, byte_to_line_idx(text, line_to_byte_idx(text, 0)));

        assert_eq!(21, line_to_byte_idx(text, byte_to_line_idx(text, 21)));
        assert_eq!(5, byte_to_line_idx(text, line_to_byte_idx(text, 5)));
    }

    #[test]
    fn line_char_round_trip() {
        let text = "\nHere\nare\nsome\nwords\n";
        assert_eq!(6, line_to_char_idx(text, char_to_line_idx(text, 6)));
        assert_eq!(2, char_to_line_idx(text, line_to_char_idx(text, 2)));

        assert_eq!(0, line_to_char_idx(text, char_to_line_idx(text, 0)));
        assert_eq!(0, char_to_line_idx(text, line_to_char_idx(text, 0)));

        assert_eq!(21, line_to_char_idx(text, char_to_line_idx(text, 21)));
        assert_eq!(5, char_to_line_idx(text, line_to_char_idx(text, 5)));
    }

    #[test]
    fn usize_flag_bytes_01() {
        let v: usize = 0xE2_09_08_A6_E2_A6_E2_09;
        assert_eq!(0x00_00_00_00_00_00_00_00, v.cmp_eq_byte(0x07));
        assert_eq!(0x00_00_01_00_00_00_00_00, v.cmp_eq_byte(0x08));
        assert_eq!(0x00_01_00_00_00_00_00_01, v.cmp_eq_byte(0x09));
        assert_eq!(0x00_00_00_01_00_01_00_00, v.cmp_eq_byte(0xA6));
        assert_eq!(0x01_00_00_00_01_00_01_00, v.cmp_eq_byte(0xE2));
    }

    #[test]
    fn usize_bytes_between_127_01() {
        let v: usize = 0x7E_09_00_A6_FF_7F_08_07;
        assert_eq!(0x01_01_00_00_00_00_01_01, v.bytes_between_127(0x00, 0x7F));
        assert_eq!(0x00_01_00_00_00_00_01_00, v.bytes_between_127(0x07, 0x7E));
        assert_eq!(0x00_01_00_00_00_00_00_00, v.bytes_between_127(0x08, 0x7E));
    }
}
