#[macro_use]
extern crate proptest;

use proptest::test_runner::Config;
use str_indices::utf16;

/// A slower, but easy-to-verify version of the library function.
fn count_surrogates_slow(text: &str) -> usize {
    text.chars().map(|c| c.len_utf16() - 1).sum()
}

/// A slower, but easy-to-verify version of the library function.
fn from_byte_idx_slow(text: &str, byte_idx: usize) -> usize {
    let mut byte_i = 0;
    let mut utf16_i = 0;

    for c in text.chars() {
        byte_i += c.len_utf8();
        if byte_i > byte_idx {
            break;
        }
        utf16_i += c.len_utf16();
        if byte_i == byte_idx {
            break;
        }
    }

    utf16_i
}

/// A slower, but easy-to-verify version of the library function.
fn to_byte_idx_slow(text: &str, utf16_idx: usize) -> usize {
    let mut byte_i = 0;
    let mut utf16_i = 0;

    for c in text.chars() {
        utf16_i += c.len_utf16();
        if utf16_i > utf16_idx {
            break;
        }
        byte_i += c.len_utf8();
        if utf16_i == utf16_idx {
            break;
        }
    }

    byte_i
}

//===========================================================================

#[cfg(miri)]
const ROUNDS: u32 = 4;
#[cfg(not(miri))]
const ROUNDS: u32 = 512;

proptest! {
    #![proptest_config(Config::with_cases(ROUNDS))]

    #[test]
    fn pt_count(ref text in "\\PC{0, 200}") {
        assert_eq!(
            from_byte_idx_slow(text, text.len()),
            utf16::count(text),
        );
    }

    #[test]
    fn pt_count_surrogates(ref text in "\\PC{0, 200}") {
        assert_eq!(
            count_surrogates_slow(text),
            utf16::count_surrogates(text),
        );
    }

    #[test]
    fn pt_from_byte_idx(ref text in "\\PC{0, 200}", idx in 0usize..300) {
        assert_eq!(
            from_byte_idx_slow(text, idx),
            utf16::from_byte_idx(text, idx),
        );
    }

    #[test]
    fn pt_to_byte_idx(ref text in "\\PC{0, 200}", idx in 0usize..300) {
        assert_eq!(
            to_byte_idx_slow(text, idx),
            utf16::to_byte_idx(text, idx),
        );
    }
}
