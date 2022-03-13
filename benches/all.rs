use criterion::{black_box, criterion_group, criterion_main, Criterion};
use str_indices::{chars, lines, lines_crlf, lines_lf, utf16};

// Texts to benchmark against.
const EN_100: &str = include_str!("text/en_100.txt");
const EN_1000: &str = include_str!("text/en_1000.txt");
const JP_100: &str = include_str!("text/jp_100.txt");
const JP_1000: &str = include_str!("text/jp_1000.txt");
const C_1000: &str = include_str!("text/c_1000.txt");

macro_rules! bench_chars {
    ($text:ident, $suite_fn_name:ident, $suite_name_str:literal) => {
        fn $suite_fn_name(c: &mut Criterion) {
            let mut group = c.benchmark_group($suite_name_str);

            group.bench_function("count", |bench| {
                bench.iter(|| {
                    black_box(chars::count($text));
                })
            });

            group.bench_function("from_byte_idx", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box(chars::from_byte_idx($text, idx));
                })
            });

            group.bench_function("to_byte_idx", |bench| {
                let idx = chars::count($text) - 1;
                bench.iter(|| {
                    black_box(chars::to_byte_idx($text, idx));
                })
            });
        }
    };
}

macro_rules! bench_chars_std {
    ($text:ident, $suite_fn_name:ident, $suite_name_str:literal) => {
        fn $suite_fn_name(c: &mut Criterion) {
            let mut group = c.benchmark_group($suite_name_str);

            group.bench_function("count", |bench| {
                bench.iter(|| {
                    black_box($text.chars().count());
                })
            });

            group.bench_function("from_byte_idx", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box({
                        let mut byte_idx = idx;
                        // Find the beginning of the code point.
                        while !$text.is_char_boundary(byte_idx) {
                            byte_idx -= 1;
                        }
                        // Count the number of chars until the
                        // char that begins at `byte_idx`.
                        (&$text[..byte_idx]).chars().count()
                    })
                })
            });

            group.bench_function("to_byte_idx", |bench| {
                let idx = chars::count($text) - 1;
                bench.iter(|| {
                    black_box($text.char_indices().skip(idx).next().unwrap().0);
                })
            });
        }
    };
}

macro_rules! bench_utf16 {
    ($text:ident, $suite_fn_name:ident, $suite_name_str:literal) => {
        fn $suite_fn_name(c: &mut Criterion) {
            let mut group = c.benchmark_group($suite_name_str);

            group.bench_function("count", |bench| {
                bench.iter(|| {
                    black_box(utf16::count($text));
                })
            });

            group.bench_function("count_surrogates", |bench| {
                bench.iter(|| {
                    black_box(utf16::count_surrogates($text));
                })
            });

            group.bench_function("from_byte_idx", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box(utf16::from_byte_idx($text, idx));
                })
            });

            group.bench_function("to_byte_idx", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box(utf16::to_byte_idx($text, idx));
                })
            });
        }
    };
}

macro_rules! bench_lines {
    ($text:ident, $suite_fn_name:ident, $suite_name_str:literal) => {
        fn $suite_fn_name(c: &mut Criterion) {
            let mut group = c.benchmark_group($suite_name_str);

            group.bench_function("count_breaks", |bench| {
                bench.iter(|| {
                    black_box(lines::count_breaks($text));
                })
            });

            group.bench_function("count_breaks_lf", |bench| {
                bench.iter(|| {
                    black_box(lines_lf::count_breaks($text));
                })
            });

            group.bench_function("count_breaks_crlf", |bench| {
                bench.iter(|| {
                    black_box(lines_crlf::count_breaks($text));
                })
            });

            group.bench_function("from_byte_idx", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box(lines::from_byte_idx($text, idx));
                })
            });

            group.bench_function("from_byte_idx_lf", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box(lines_lf::from_byte_idx($text, idx));
                })
            });

            group.bench_function("from_byte_idx_crlf", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box(lines_crlf::from_byte_idx($text, idx));
                })
            });

            group.bench_function("to_byte_idx", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box(lines::to_byte_idx($text, idx));
                })
            });

            group.bench_function("to_byte_idx_lf", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box(lines_lf::to_byte_idx($text, idx));
                })
            });

            group.bench_function("to_byte_idx_crlf", |bench| {
                let idx = $text.len() - 1;
                bench.iter(|| {
                    black_box(lines_crlf::to_byte_idx($text, idx));
                })
            });
        }
    };
}

//-------------------------------------------------------------

bench_chars!(EN_100, chars_english_100, "chars_english_100");
bench_chars!(EN_1000, chars_english_1000, "chars_english_1000");
bench_chars!(JP_100, chars_japanese_100, "chars_japanese_100");
bench_chars!(JP_1000, chars_japanese_1000, "chars_japanese_1000");
bench_chars!(C_1000, chars_c_source_1000, "chars_c_source_1000");

bench_chars_std!(EN_100, chars_english_100_std, "chars_english_100_std");
bench_chars_std!(EN_1000, chars_english_1000_std, "chars_english_1000_std");
bench_chars_std!(JP_100, chars_japanese_100_std, "chars_japanese_100_std");
bench_chars_std!(JP_1000, chars_japanese_1000_std, "chars_japanese_1000_std");
bench_chars_std!(C_1000, chars_c_source_1000_std, "chars_c_source_1000_std");

bench_utf16!(EN_100, utf16_english_100, "utf16_english_100");
bench_utf16!(EN_1000, utf16_english_1000, "utf16_english_1000");
bench_utf16!(JP_100, utf16_japanese_100, "utf16_japanese_100");
bench_utf16!(JP_1000, utf16_japanese_1000, "utf16_japanese_1000");
bench_utf16!(C_1000, utf16_c_source_1000, "utf16_c_source_1000");

bench_lines!(EN_100, lines_english_100, "lines_english_100");
bench_lines!(EN_1000, lines_english_1000, "lines_english_1000");
bench_lines!(JP_100, lines_japanese_100, "lines_japanese_100");
bench_lines!(JP_1000, lines_japanese_1000, "lines_japanese_1000");
bench_lines!(C_1000, lines_c_source_1000, "lines_c_source_1000");

//-------------------------------------------------------------

criterion_group!(
    benches,
    chars_english_100,
    chars_english_1000,
    chars_japanese_100,
    chars_japanese_1000,
    chars_c_source_1000,
    chars_english_100_std,
    chars_english_1000_std,
    chars_japanese_100_std,
    chars_japanese_1000_std,
    chars_c_source_1000_std,
    utf16_english_100,
    utf16_english_1000,
    utf16_japanese_100,
    utf16_japanese_1000,
    utf16_c_source_1000,
    lines_english_100,
    lines_english_1000,
    lines_japanese_100,
    lines_japanese_1000,
    lines_c_source_1000,
);
criterion_main!(benches);
