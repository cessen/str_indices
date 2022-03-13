#![no_main]

use libfuzzer_sys::fuzz_target;
use str_indices::{chars, lines, lines_crlf, lines_lf, utf16};

fuzz_target!(|data: (String, usize, bool)| {
    let text = &data.0[..];

    // Encourage the index to be in-bounds at
    // least a good chunk of the time.
    let idx = if data.2 {
        data.1
    } else {
        data.1 % text.len().max(1)
    };

    chars::count(text);
    chars::from_byte_idx(text, idx);
    chars::to_byte_idx(text, idx);

    utf16::count(text);
    utf16::count_surrogates(text);
    utf16::from_byte_idx(text, idx);
    utf16::to_byte_idx(text, idx);

    lines::count_breaks(text);
    lines::from_byte_idx(text, idx);
    lines::to_byte_idx(text, idx);

    lines_lf::count_breaks(text);
    lines_lf::from_byte_idx(text, idx);
    lines_lf::to_byte_idx(text, idx);

    lines_crlf::count_breaks(text);
    lines_crlf::from_byte_idx(text, idx);
    lines_crlf::to_byte_idx(text, idx);
});
