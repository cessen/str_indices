#[macro_use]
extern crate proptest;

use proptest::test_runner::Config;
use str_indices::lines_lf;

/// A slower, but easy-to-verify version of the library function.
fn from_byte_idx_slow(text: &str, byte_idx: usize) -> usize {
    let mut byte_itr = text.bytes();
    let mut i = 0;
    let mut line_count = 0;

    while let Some(byte) = byte_itr.next() {
        if i >= byte_idx {
            break;
        }
        if byte == 0x0A {
            line_count += 1;
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
        if byte == 0x0A {
            line_count += 1;
        }
        i += 1;
    }

    i
}

//===========================================================================

proptest! {
    #![proptest_config(Config::with_cases(512))]

    #[test]
    fn pt_count_breaks(ref text in "[aあ🐸\\u{000A}]{0, 200}") {
        assert_eq!(
            from_byte_idx_slow(text, text.len()),
            lines_lf::count_breaks(text),
        );
    }

    #[test]
    fn pt_from_byte_idx(ref text in "[aあ🐸\\u{000A}]{0, 200}", idx in 0usize..400) {
        assert_eq!(
            from_byte_idx_slow(text, idx),
            lines_lf::from_byte_idx(text, idx),
        );
    }

    #[test]
    fn pt_to_byte_idx(ref text in "[aあ🐸\\u{000A}]{0, 200}", idx in 0usize..300) {
        assert_eq!(
            to_byte_idx_slow(text, idx),
            lines_lf::to_byte_idx(text, idx),
        );
    }
}
