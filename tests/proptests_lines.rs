#[macro_use]
extern crate proptest;

use proptest::test_runner::Config;
use str_indices::lines;

/// A slower, but easy-to-verify version of the library function.
fn from_byte_idx_slow(text: &str, byte_idx: usize) -> usize {
    let mut byte_itr = text.bytes();
    let mut i = 0;
    let mut line_count = 0;

    while let Some(byte) = byte_itr.next() {
        if i >= byte_idx {
            break;
        }

        match byte {
            0x0A | 0x0B | 0x0C => {
                line_count += 1;
            }
            0x0D => {
                // Check for a following LF.  By cloning the itr, we're
                // peeking without actually stepping the original itr.
                if let Some(0x0A) = byte_itr.clone().next() {
                    // Do nothing.  The CRLF will be properly counted
                    // on the next iteration if the LF is behind
                    // byte_idx.
                } else {
                    // There's no following LF, so the stand-alone CR is a
                    // line ending itself.
                    line_count += 1;
                }
            }
            0xC2 => {
                if (i + 1) < byte_idx {
                    i += 1;
                    if let Some(0x85) = byte_itr.next() {
                        line_count += 1;
                    }
                }
            }
            0xE2 => {
                if (i + 2) < byte_idx {
                    i += 2;
                    let byte2 = byte_itr.next().unwrap();
                    let byte3 = byte_itr.next().unwrap() >> 1;
                    if byte2 == 0x80 && byte3 == 0x54 {
                        line_count += 1;
                    }
                }
            }
            _ => {}
        }

        i += 1;
    }

    line_count
}

/// A slower, but easy-to-verify version of the library function.
fn to_byte_idx_slow(text: &str, line_idx: usize) -> usize {
    let mut byte_itr = text.bytes();
    let mut i = 0;
    let mut line_count = 0;

    while let Some(byte) = byte_itr.next() {
        if line_count == line_idx {
            break;
        }

        match byte {
            0x0A | 0x0B | 0x0C => {
                line_count += 1;
            }
            0x0D => {
                // Check for a following LF.  By cloning the itr, we're
                // peeking without actually stepping the original itr.
                if let Some(0x0A) = byte_itr.clone().next() {
                    // Skip the LF, since it's part of the CRLF
                    // pair.
                    i += 1;
                    byte_itr.next();
                }
                line_count += 1;
            }
            0xC2 => {
                i += 1;
                if let Some(0x85) = byte_itr.next() {
                    line_count += 1;
                }
            }
            0xE2 => {
                i += 2;
                let byte2 = byte_itr.next().unwrap();
                let byte3 = byte_itr.next().unwrap() >> 1;
                if byte2 == 0x80 && byte3 == 0x54 {
                    line_count += 1;
                }
            }
            _ => {}
        }

        i += 1;
    }

    i
}

//===========================================================================

#[cfg(miri)]
const ROUNDS: u32 = 4;
#[cfg(not(miri))]
const ROUNDS: u32 = 512;

proptest! {
    #![proptest_config(Config::with_cases(ROUNDS))]

    #[test]
    fn pt_count_breaks(ref text in "[aあ🐸\\u{000A}\\u{000B}\\u{000C}\\u{000D}\\u{0085}\\u{2028}\\u{2029}]{0, 200}") {
        assert_eq!(
            from_byte_idx_slow(text, text.len()),
            lines::count_breaks(text),
        );
    }

    #[test]
    fn pt_from_byte_idx(ref text in "[aあ🐸\\u{000A}\\u{000B}\\u{000C}\\u{000D}\\u{0085}\\u{2028}\\u{2029}]{0, 200}", idx in 0usize..400) {
        assert_eq!(
            from_byte_idx_slow(text, idx),
            lines::from_byte_idx(text, idx),
        );
    }

    #[test]
    fn pt_to_byte_idx(ref text in "[aあ🐸\\u{000A}\\u{000B}\\u{000C}\\u{000D}\\u{0085}\\u{2028}\\u{2029}]{0, 200}", idx in 0usize..300) {
        assert_eq!(
            to_byte_idx_slow(text, idx),
            lines::to_byte_idx(text, idx),
        );
    }
}
