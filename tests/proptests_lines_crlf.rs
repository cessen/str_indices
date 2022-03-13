#[macro_use]
extern crate proptest;

use proptest::test_runner::Config;
use str_indices::lines_crlf;

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
            0x0A => {
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
            0x0A => {
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
            _ => {}
        }

        i += 1;
    }

    i
}

//===========================================================================

proptest! {
    #![proptest_config(Config::with_cases(512))]

    #[test]
    fn pt_count_breaks(ref text in "[aあ🐸\\u{000A}\\u{000D}]{0, 200}") {
        assert_eq!(
            from_byte_idx_slow(text, text.len()),
            lines_crlf::count_breaks(text),
        );
    }

    #[test]
    fn pt_from_byte_idx(ref text in "[aあ🐸\\u{000A}\\u{000D}]{0, 200}", idx in 0usize..400) {
        assert_eq!(
            from_byte_idx_slow(text, idx),
            lines_crlf::from_byte_idx(text, idx),
        );
    }

    #[test]
    fn pt_to_byte_idx(ref text in "[aあ🐸\\u{000A}\\u{000D}]{0, 200}", idx in 0usize..300) {
        assert_eq!(
            to_byte_idx_slow(text, idx),
            lines_crlf::to_byte_idx(text, idx),
        );
    }
}
