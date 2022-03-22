use std::fs;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use str_indices::{chars, lines, lines_crlf, lines_lf, utf16};

fn all(c: &mut Criterion) {
    // Load benchmark strings.
    let test_strings: Vec<(&str, String)> = vec![
        ("en_0001", "E".into()),
        (
            "en_0010",
            fs::read_to_string("benches/text/en_10.txt").expect("Cannot find benchmark text."),
        ),
        (
            "en_0100",
            fs::read_to_string("benches/text/en_100.txt").expect("Cannot find benchmark text."),
        ),
        (
            "en_1000",
            fs::read_to_string("benches/text/en_1000.txt").expect("Cannot find benchmark text."),
        ),
        ("jp_0003", "日".into()),
        (
            "jp_0102",
            fs::read_to_string("benches/text/jp_102.txt").expect("Cannot find benchmark text."),
        ),
        (
            "jp_1001",
            fs::read_to_string("benches/text/jp_1001.txt").expect("Cannot find benchmark text."),
        ),
    ];

    //---------------------------------------------------------
    // Chars.

    // chars::count()
    {
        let mut group = c.benchmark_group("chars::count");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                bench.iter(|| {
                    black_box(chars::count(text));
                })
            });
        }
    }
    {
        // Equivalent implementations using stdlib functions,
        // for performance comparisons.
        let mut group = c.benchmark_group("chars::count_std");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                bench.iter(|| {
                    black_box(text.chars().count());
                })
            });
        }
    }

    // chars::from_byte_idx()
    {
        let mut group = c.benchmark_group("chars::from_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = text.len();
                bench.iter(|| {
                    black_box(chars::from_byte_idx(text, idx));
                })
            });
        }
    }
    {
        // Equivalent implementations using stdlib functions,
        // for performance comparisons.
        let mut group = c.benchmark_group("chars::from_byte_idx_std");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(format!("std::{}", text_name), |bench| {
                let idx = text.len();
                bench.iter(|| {
                    black_box({
                        let mut byte_idx = idx;
                        // Find the beginning of the code point.
                        while !text.is_char_boundary(byte_idx) {
                            byte_idx -= 1;
                        }
                        // Count the number of chars until the
                        // char that begins at `byte_idx`.
                        (&text[..byte_idx]).chars().count()
                    })
                })
            });
        }
    }

    // chars::to_byte_idx()
    {
        let mut group = c.benchmark_group("chars::to_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = chars::count(text);
                bench.iter(|| {
                    black_box(chars::to_byte_idx(text, idx));
                })
            });
        }
    }
    {
        // Equivalent implementations using stdlib functions,
        // for performance comparisons.
        let mut group = c.benchmark_group("chars::to_byte_idx_std");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(format!("std::{}", text_name), |bench| {
                let idx = chars::count(text) - 1; // Minus 1 so we can unwrap below.
                bench.iter(|| {
                    black_box(text.char_indices().skip(idx).next().unwrap().0);
                })
            });
        }
    }

    //---------------------------------------------------------
    // UTF16.

    // utf16::count()
    {
        let mut group = c.benchmark_group("utf16::count");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                bench.iter(|| {
                    black_box(utf16::count(text));
                })
            });
        }
    }

    // utf16::count_surrogates()
    {
        let mut group = c.benchmark_group("utf16::count_surrogates");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                bench.iter(|| {
                    black_box(utf16::count_surrogates(text));
                })
            });
        }
    }

    // utf16::from_byte_idx()
    {
        let mut group = c.benchmark_group("utf16::from_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = text.len();
                bench.iter(|| {
                    black_box(utf16::from_byte_idx(text, idx));
                })
            });
        }
    }

    // utf16::to_byte_idx()
    {
        let mut group = c.benchmark_group("utf16::to_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = utf16::count(text);
                bench.iter(|| {
                    black_box(utf16::to_byte_idx(text, idx));
                })
            });
        }
    }

    //---------------------------------------------------------
    // Lines (unicode).

    // lines::count_breaks()
    {
        let mut group = c.benchmark_group("lines::count_breaks");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                bench.iter(|| {
                    black_box(lines::count_breaks(text));
                })
            });
        }
    }

    // lines::from_byte_idx()
    {
        let mut group = c.benchmark_group("lines::from_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = text.len();
                bench.iter(|| {
                    black_box(lines::from_byte_idx(text, idx));
                })
            });
        }
    }

    // lines::to_byte_idx()
    {
        let mut group = c.benchmark_group("lines::to_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = lines::count_breaks(text) + 1;
                bench.iter(|| {
                    black_box(lines::to_byte_idx(text, idx));
                })
            });
        }
    }

    //---------------------------------------------------------
    // Lines (LF).

    // lines_lf::count_breaks()
    {
        let mut group = c.benchmark_group("lines_lf::count_breaks");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                bench.iter(|| {
                    black_box(lines_lf::count_breaks(text));
                })
            });
        }
    }
    {
        // Version implemented with stdlib functions,
        // for performance comparisons.  Note: this
        // isn't exactly identical in behavior, since
        // stdlib ignores document-final line breaks.
        // But it should be close enough for perf
        // comparisons.
        let mut group = c.benchmark_group("lines_lf::count_breaks_std");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                bench.iter(|| {
                    black_box(text.lines().count());
                })
            });
        }
    }

    // lines_lf::from_byte_idx()
    {
        let mut group = c.benchmark_group("lines_lf::from_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = text.len();
                bench.iter(|| {
                    black_box(lines_lf::from_byte_idx(text, idx));
                })
            });
        }
    }

    // lines_lf::to_byte_idx()
    {
        let mut group = c.benchmark_group("lines_lf::to_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = lines_lf::count_breaks(text) + 1;
                bench.iter(|| {
                    black_box(lines_lf::to_byte_idx(text, idx));
                })
            });
        }
    }

    //---------------------------------------------------------
    // Lines (CRLF).

    // lines_crlf::count_breaks()
    {
        let mut group = c.benchmark_group("lines_crlf::count_breaks");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                bench.iter(|| {
                    black_box(lines_crlf::count_breaks(text));
                })
            });
        }
    }

    // lines_crlf::from_byte_idx()
    {
        let mut group = c.benchmark_group("lines_crlf::from_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = text.len();
                bench.iter(|| {
                    black_box(lines_crlf::from_byte_idx(text, idx));
                })
            });
        }
    }

    // lines_crlf::to_byte_idx()
    {
        let mut group = c.benchmark_group("lines_crlf::to_byte_idx");
        for (text_name, text) in test_strings.iter() {
            group.bench_function(*text_name, |bench| {
                let idx = lines_crlf::count_breaks(text) + 1;
                bench.iter(|| {
                    black_box(lines_crlf::to_byte_idx(text, idx));
                })
            });
        }
    }
}

//-------------------------------------------------------------

criterion_group!(benches, all,);
criterion_main!(benches);
