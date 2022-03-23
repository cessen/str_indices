#[macro_use]
extern crate proptest;

use proptest::test_runner::Config;
use str_indices::chars;

/// A slower, but easy-to-verify version of the library function.
fn from_byte_idx_slow(text: &str, byte_idx: usize) -> usize {
    let mut byte_i = 0;
    let mut char_i = 0;

    while byte_i < byte_idx && byte_i < text.len() {
        byte_i += 1;
        if text.is_char_boundary(byte_i) {
            char_i += 1;
        }
    }

    char_i
}

/// A slower, but easy-to-verify version of the library function.
fn to_byte_idx_slow(text: &str, char_idx: usize) -> usize {
    let mut byte_i = 0;
    let mut char_i = 0;

    while char_i < char_idx && byte_i < text.len() {
        byte_i += 1;
        if text.is_char_boundary(byte_i) {
            char_i += 1;
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
            chars::count(text),
        );
    }

    #[test]
    fn pt_from_byte_idx(ref text in "\\PC{0, 200}", idx in 0usize..300) {
        assert_eq!(
            from_byte_idx_slow(text, idx),
            chars::from_byte_idx(text, idx),
        );
    }

    #[test]
    fn pt_to_byte_idx(ref text in "\\PC{0, 200}", idx in 0usize..300) {
        assert_eq!(
            to_byte_idx_slow(text, idx),
            chars::to_byte_idx(text, idx),
        );
    }
}
