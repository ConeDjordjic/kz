use memchr::memmem::Finder;
use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

const CHUNK_SIZE: usize = 1024 * 1024;
const PARALLEL_THRESHOLD: usize = 512 * 1024;

pub fn count_lines(data: &[u8]) -> usize {
    if data.len() < PARALLEL_THRESHOLD {
        return memchr::memchr_iter(b'\n', data).count();
    }

    data.par_chunks(CHUNK_SIZE)
        .map(|chunk| memchr::memchr_iter(b'\n', chunk).count())
        .sum()
}

pub fn count_blank_lines(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    if data.len() < PARALLEL_THRESHOLD {
        return count_blank_lines_chunk(data);
    }

    let boundaries = find_line_boundaries(data, CHUNK_SIZE);
    boundaries
        .par_windows(2)
        .map(|w| count_blank_lines_chunk(&data[w[0]..w[1]]))
        .sum()
}

fn count_blank_lines_chunk(data: &[u8]) -> usize {
    let mut count = 0;
    let mut line_start = 0;

    for pos in memchr::memchr_iter(b'\n', data) {
        if data[line_start..pos]
            .iter()
            .all(|&b| b.is_ascii_whitespace())
        {
            count += 1;
        }
        line_start = pos + 1;
    }

    if line_start < data.len() && data[line_start..].iter().all(|&b| b.is_ascii_whitespace()) {
        count += 1;
    }

    count
}

fn find_line_boundaries(data: &[u8], chunk_size: usize) -> Vec<usize> {
    let mut boundaries = vec![0];
    let mut pos = chunk_size;

    while pos < data.len() {
        if let Some(nl) = memchr::memchr(b'\n', &data[pos..]) {
            pos += nl + 1;
        } else {
            pos = data.len();
        }
        boundaries.push(pos);
        pos += chunk_size;
    }

    if *boundaries.last().unwrap() != data.len() {
        boundaries.push(data.len());
    }

    boundaries
}

pub fn count_all_words(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    if data.len() < PARALLEL_THRESHOLD {
        return count_words_in_chunk(data);
    }

    let chunk_boundaries = find_utf8_chunk_boundaries(data, CHUNK_SIZE);
    let count: usize = chunk_boundaries
        .par_windows(2)
        .map(|window| {
            let chunk = &data[window[0]..window[1]];
            count_words_in_chunk(chunk)
        })
        .sum();

    let mut overcounted = 0;
    for window in chunk_boundaries.windows(2) {
        let boundary = window[1];
        if boundary > 0 && boundary < data.len() {
            let prev_byte = data[boundary - 1];
            let curr_byte = data[boundary];
            if !prev_byte.is_ascii_whitespace() && !curr_byte.is_ascii_whitespace() {
                overcounted += 1;
            }
        }
    }

    count.saturating_sub(overcounted)
}

fn find_utf8_chunk_boundaries(data: &[u8], chunk_size: usize) -> Vec<usize> {
    let mut boundaries = vec![0];
    let mut pos = chunk_size;

    while pos < data.len() {
        pos = find_utf8_boundary(data, pos);
        boundaries.push(pos);
        pos += chunk_size;
    }

    if *boundaries.last().unwrap() != data.len() {
        boundaries.push(data.len());
    }

    boundaries
}

fn find_utf8_boundary(data: &[u8], pos: usize) -> usize {
    if pos >= data.len() {
        return data.len();
    }

    let mut p = pos;
    while p < data.len() && (data[p] & 0xC0) == 0x80 {
        p += 1;
    }
    p
}

#[inline]
fn count_words_in_chunk(chunk: &[u8]) -> usize {
    if let Ok(text) = std::str::from_utf8(chunk) {
        let mut count = 0;
        let mut in_word = false;

        for c in text.chars() {
            if c.is_whitespace() {
                in_word = false;
            } else if !in_word {
                count += 1;
                in_word = true;
            }
        }

        count
    } else {
        let mut count = 0;
        let mut in_word = false;

        for &byte in chunk {
            if byte.is_ascii_whitespace() {
                in_word = false;
            } else if !in_word {
                count += 1;
                in_word = true;
            }
        }

        count
    }
}

pub fn count_pattern(data: &[u8], pattern: &[u8]) -> usize {
    if data.is_empty() || pattern.is_empty() {
        return 0;
    }

    let finder = Finder::new(pattern);

    if data.len() < PARALLEL_THRESHOLD {
        return finder.find_iter(data).count();
    }

    let num_chunks = data.len().div_ceil(CHUNK_SIZE);
    let count: usize = (0..num_chunks)
        .into_par_iter()
        .map(|i| {
            let start = i * CHUNK_SIZE;
            let end = ((i + 1) * CHUNK_SIZE).min(data.len());
            let chunk = &data[start..end];
            finder.find_iter(chunk).count()
        })
        .sum();

    let mut boundary_matches = 0;
    for i in 1..num_chunks {
        let boundary = i * CHUNK_SIZE;
        let search_start = boundary.saturating_sub(pattern.len() - 1);
        let search_end = (boundary + pattern.len() - 1).min(data.len());

        if search_start >= search_end {
            continue;
        }

        let region = &data[search_start..search_end];
        for pos in finder.find_iter(region) {
            let abs_start = search_start + pos;
            if abs_start < boundary && abs_start + pattern.len() > boundary {
                boundary_matches += 1;
            }
        }
    }

    count + boundary_matches
}

pub fn count_chars(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    if data.len() < PARALLEL_THRESHOLD {
        return std::str::from_utf8(data)
            .map(|s| s.chars().count())
            .unwrap_or(data.len());
    }

    let chunk_boundaries = find_utf8_chunk_boundaries(data, CHUNK_SIZE);
    chunk_boundaries
        .par_windows(2)
        .map(|window| {
            let chunk = &data[window[0]..window[1]];
            std::str::from_utf8(chunk)
                .map(|s| s.chars().count())
                .unwrap_or(chunk.len())
        })
        .sum()
}

pub fn max_line_length(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    if data.len() < PARALLEL_THRESHOLD {
        return max_line_length_chunk(data);
    }

    let boundaries = find_line_boundaries(data, CHUNK_SIZE);
    boundaries
        .par_windows(2)
        .map(|w| max_line_length_chunk(&data[w[0]..w[1]]))
        .max()
        .unwrap_or(0)
}

fn max_line_length_chunk(data: &[u8]) -> usize {
    let mut max_len = 0;
    let mut prev = 0;

    for pos in memchr::memchr_iter(b'\n', data) {
        let mut end = pos;
        if end > prev && data[end - 1] == b'\r' {
            end -= 1;
        }
        max_len = max_len.max(end - prev);
        prev = pos + 1;
    }

    if prev < data.len() {
        let mut end = data.len();
        if end > prev && data[end - 1] == b'\r' {
            end -= 1;
        }
        max_len = max_len.max(end - prev);
    }

    max_len
}

pub fn is_binary(data: &[u8]) -> bool {
    let sample_size = data.len().min(8192);
    let sample = &data[..sample_size];
    memchr::memchr(0, sample).is_some()
}

pub fn count_unique_words(data: &[u8]) -> usize {
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if data.len() < PARALLEL_THRESHOLD {
        let words: HashSet<&str> = text
            .split(|c: char| c.is_whitespace())
            .filter(|w| !w.is_empty())
            .collect();
        return words.len();
    }

    let boundaries = find_line_boundaries(data, CHUNK_SIZE);

    let local_sets: Vec<HashSet<&str>> = boundaries
        .par_windows(2)
        .map(|window| {
            let chunk = &data[window[0]..window[1]];
            let chunk_text = std::str::from_utf8(chunk).unwrap_or("");
            let mut local_set = HashSet::new();
            for word in chunk_text.split(|c: char| c.is_whitespace()) {
                if !word.is_empty() {
                    local_set.insert(word);
                }
            }
            local_set
        })
        .collect();

    let mut final_set = HashSet::new();
    for set in local_sets {
        final_set.extend(set);
    }

    final_set.len()
}

pub struct Statistics {
    pub mean_line_length: f64,
    pub median_line_length: usize,
    pub std_dev: f64,
    pub min_line_length: usize,
    pub max_line_length: usize,
    pub empty_lines: usize,
}

pub fn calculate_statistics(data: &[u8]) -> Statistics {
    if data.is_empty() {
        return Statistics {
            mean_line_length: 0.0,
            median_line_length: 0,
            std_dev: 0.0,
            min_line_length: 0,
            max_line_length: 0,
            empty_lines: 0,
        };
    }

    let line_lengths = if data.len() < PARALLEL_THRESHOLD {
        collect_line_lengths_chunk(data)
    } else {
        let boundaries = find_line_boundaries(data, CHUNK_SIZE);

        boundaries
            .par_windows(2)
            .flat_map(|w| collect_line_lengths_chunk(&data[w[0]..w[1]]))
            .collect()
    };

    if line_lengths.is_empty() {
        return Statistics {
            mean_line_length: 0.0,
            median_line_length: 0,
            std_dev: 0.0,
            min_line_length: 0,
            max_line_length: 0,
            empty_lines: 0,
        };
    }

    let empty_lines = line_lengths.iter().filter(|&&l| l == 0).count();
    let sum: usize = line_lengths.iter().sum();
    let mean = sum as f64 / line_lengths.len() as f64;

    let variance: f64 = if data.len() < PARALLEL_THRESHOLD {
        line_lengths
            .iter()
            .map(|&len| {
                let diff = len as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
    } else {
        line_lengths
            .par_iter()
            .map(|&len| {
                let diff = len as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
    } / line_lengths.len() as f64;

    let std_dev = variance.sqrt();

    let mut sorted = line_lengths;
    sorted.sort_unstable();

    let median = if sorted.len() % 2 == 0 {
        let mid = sorted.len() / 2;
        (sorted[mid - 1] + sorted[mid]) / 2
    } else {
        sorted[sorted.len() / 2]
    };

    Statistics {
        mean_line_length: mean,
        median_line_length: median,
        std_dev,
        min_line_length: sorted[0],
        max_line_length: sorted[sorted.len() - 1],
        empty_lines,
    }
}

fn collect_line_lengths_chunk(data: &[u8]) -> Vec<usize> {
    let mut lengths = Vec::new();
    let mut prev = 0;

    for pos in memchr::memchr_iter(b'\n', data) {
        let mut end = pos;
        if end > prev && data[end - 1] == b'\r' {
            end -= 1;
        }
        lengths.push(end - prev);
        prev = pos + 1;
    }

    if prev < data.len() {
        let mut end = data.len();
        if end > prev && data[end - 1] == b'\r' {
            end -= 1;
        }
        lengths.push(end - prev);
    }

    lengths
}

pub fn generate_histogram(data: &[u8]) -> HashMap<usize, usize> {
    if data.is_empty() {
        return HashMap::new();
    }

    if data.len() < PARALLEL_THRESHOLD {
        return generate_histogram_chunk(data);
    }

    let boundaries = find_line_boundaries(data, CHUNK_SIZE);

    let maps: Vec<HashMap<usize, usize>> = boundaries
        .par_windows(2)
        .map(|w| generate_histogram_chunk(&data[w[0]..w[1]]))
        .collect();

    let mut histogram = HashMap::new();
    for map in maps {
        for (bucket, count) in map {
            *histogram.entry(bucket).or_insert(0) += count;
        }
    }

    histogram
}

fn generate_histogram_chunk(data: &[u8]) -> HashMap<usize, usize> {
    let mut histogram = HashMap::new();
    let mut prev = 0;

    for pos in memchr::memchr_iter(b'\n', data) {
        let mut end = pos;
        if end > prev && data[end - 1] == b'\r' {
            end -= 1;
        }
        let bucket = ((end - prev) / 10) * 10;
        *histogram.entry(bucket).or_insert(0) += 1;
        prev = pos + 1;
    }

    if prev < data.len() {
        let mut end = data.len();
        if end > prev && data[end - 1] == b'\r' {
            end -= 1;
        }
        let bucket = ((end - prev) / 10) * 10;
        *histogram.entry(bucket).or_insert(0) += 1;
    }

    histogram
}

fn find_comment_marker(s: &str, marker: &str, require_whitespace_before: bool) -> Option<usize> {
    let mut start = 0;
    while let Some(pos) = s[start..].find(marker) {
        let abs_pos = start + pos;
        if !require_whitespace_before
            || abs_pos == 0
            || s[..abs_pos]
                .chars()
                .last()
                .is_none_or(|c| c.is_whitespace())
        {
            return Some(abs_pos);
        }
        start = abs_pos + 1;
    }
    None
}

pub fn filter_code_comments(data: &[u8]) -> Vec<u8> {
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return data.to_vec(),
    };

    let mut result = Vec::new();
    let mut in_multiline_c_comment = false;
    let mut in_python_docstring = false;
    let mut docstring_marker: &str = "";

    for line in text.lines() {
        let mut current = line;
        let mut line_output = String::new();

        while !current.is_empty() {
            if in_multiline_c_comment {
                if let Some(pos) = current.find("*/") {
                    in_multiline_c_comment = false;
                    current = &current[pos + 2..];
                } else {
                    break;
                }
            } else if in_python_docstring {
                if let Some(pos) = current.find(docstring_marker) {
                    in_python_docstring = false;
                    current = &current[pos + docstring_marker.len()..];
                } else {
                    break;
                }
            } else {
                let markers: [(Option<usize>, &str); 6] = [
                    (find_comment_marker(current, "//", true), "single_slash"),
                    (find_comment_marker(current, "#", true), "single_hash"),
                    (find_comment_marker(current, "--", true), "single_dash"),
                    (find_comment_marker(current, "/*", true), "multi"),
                    (current.find("\"\"\""), "doc_double"),
                    (current.find("'''"), "doc_single"),
                ];

                let earliest = markers
                    .into_iter()
                    .filter_map(|(pos, kind)| pos.map(|p| (p, kind)))
                    .min_by_key(|(p, _)| *p);

                if let Some((pos, marker_type)) = earliest {
                    line_output.push_str(&current[..pos]);

                    match marker_type {
                        "single_slash" | "single_hash" | "single_dash" => {
                            break;
                        }
                        "multi" => {
                            let after = &current[pos + 2..];
                            if let Some(end_pos) = after.find("*/") {
                                current = &after[end_pos + 2..];
                            } else {
                                in_multiline_c_comment = true;
                                break;
                            }
                        }
                        "doc_double" => {
                            let after = &current[pos + 3..];
                            if let Some(end_pos) = after.find("\"\"\"") {
                                current = &after[end_pos + 3..];
                            } else {
                                docstring_marker = "\"\"\"";
                                in_python_docstring = true;
                                break;
                            }
                        }
                        "doc_single" => {
                            let after = &current[pos + 3..];
                            if let Some(end_pos) = after.find("'''") {
                                current = &after[end_pos + 3..];
                            } else {
                                docstring_marker = "'''";
                                in_python_docstring = true;
                                break;
                            }
                        }
                        _ => unreachable!(),
                    }
                } else {
                    line_output.push_str(current);
                    break;
                }
            }
        }

        let trimmed = line_output.trim_end();
        if !trimmed.trim_start().is_empty() {
            result.extend_from_slice(trimmed.as_bytes());
            result.push(b'\n');
        }
    }

    result
}

pub fn filter_markdown_code(data: &[u8]) -> Vec<u8> {
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return data.to_vec(),
    };

    let mut result = Vec::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        let filtered_line = filter_inline_code(line);
        result.extend_from_slice(filtered_line.as_bytes());
        result.push(b'\n');
    }

    result
}

fn filter_inline_code(line: &str) -> String {
    let mut result = String::new();
    let mut in_code = false;

    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
        } else if !in_code {
            result.push(c);
        }
    }

    result
}

pub fn decode_to_utf8<'a>(data: &'a [u8], encoding_name: Option<&str>) -> Cow<'a, [u8]> {
    use chardetng::EncodingDetector;
    use encoding_rs::Encoding;

    let encoding = if let Some(name) = encoding_name {
        Encoding::for_label(name.as_bytes()).unwrap_or(encoding_rs::UTF_8)
    } else {
        let mut detector = EncodingDetector::new();
        detector.feed(data, true);
        detector.guess(None, true)
    };

    if encoding == encoding_rs::UTF_8 {
        return Cow::Borrowed(data);
    }

    let (decoded, _, _) = encoding.decode(data);
    Cow::Owned(decoded.into_owned().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_lines_empty() {
        assert_eq!(count_lines(b""), 0);
    }

    #[test]
    fn test_count_lines_single() {
        assert_eq!(count_lines(b"hello\n"), 1);
    }

    #[test]
    fn test_count_lines_multiple() {
        assert_eq!(count_lines(b"line1\nline2\nline3\n"), 3);
    }

    #[test]
    fn test_count_lines_no_trailing_newline() {
        assert_eq!(count_lines(b"line1\nline2"), 1);
    }

    #[test]
    fn test_count_words_empty() {
        assert_eq!(count_all_words(b""), 0);
    }

    #[test]
    fn test_count_words_single() {
        assert_eq!(count_all_words(b"hello"), 1);
    }

    #[test]
    fn test_count_words_multiple() {
        assert_eq!(count_all_words(b"hello world foo bar"), 4);
    }

    #[test]
    fn test_count_words_multiple_spaces() {
        assert_eq!(count_all_words(b"hello    world"), 2);
    }

    #[test]
    fn test_count_words_newlines() {
        assert_eq!(count_all_words(b"hello\nworld\nfoo"), 3);
    }

    #[test]
    fn test_count_words_mixed_whitespace() {
        assert_eq!(count_all_words(b"hello\t\nworld  \r\nfoo"), 3);
    }

    #[test]
    fn test_count_words_unicode_whitespace() {
        let text = "hello\u{00A0}world";
        assert_eq!(count_all_words(text.as_bytes()), 2);

        let text2 = "hello\u{2003}world";
        assert_eq!(count_all_words(text2.as_bytes()), 2);
    }

    #[test]
    fn test_count_pattern_empty_data() {
        assert_eq!(count_pattern(b"", b"test"), 0);
    }

    #[test]
    fn test_count_pattern_empty_pattern() {
        assert_eq!(count_pattern(b"test", b""), 0);
    }

    #[test]
    fn test_count_pattern_single_occurrence() {
        assert_eq!(count_pattern(b"hello world", b"world"), 1);
    }

    #[test]
    fn test_count_pattern_multiple_occurrences() {
        assert_eq!(count_pattern(b"foo bar foo baz foo", b"foo"), 3);
    }

    #[test]
    fn test_count_pattern_non_overlapping() {
        assert_eq!(count_pattern(b"aaa", b"aa"), 1);
        assert_eq!(count_pattern(b"aaaa", b"aa"), 2);
    }

    #[test]
    fn test_count_pattern_no_match() {
        assert_eq!(count_pattern(b"hello world", b"xyz"), 0);
    }

    #[test]
    fn test_count_pattern_byte_pattern() {
        assert_eq!(count_pattern(b"a\nb\nc\n", b"\n"), 3);
    }

    #[test]
    fn test_large_data_parallel() {
        let large_text = "word ".repeat(200_000);
        let bytes = large_text.as_bytes();

        assert_eq!(count_all_words(bytes), 200_000);

        let large_lines = b"line\n".repeat(200_000);
        assert_eq!(count_lines(&large_lines), 200_000);
    }

    #[test]
    fn test_chunk_boundary_words() {
        let chunk_size = CHUNK_SIZE;
        let mut data = vec![b'a'; chunk_size - 1];
        data.push(b'b');
        data.push(b'c');

        assert_eq!(count_all_words(&data), 1);

        data[chunk_size - 1] = b' ';
        assert_eq!(count_all_words(&data), 2);
    }

    #[test]
    fn test_chunk_boundary_pattern() {
        let chunk_size = CHUNK_SIZE;
        let pattern = b"boundary";
        let mut data = vec![b'x'; chunk_size - 4];
        data.extend_from_slice(pattern);
        data.extend_from_slice(b"yyyyyy");

        assert_eq!(count_pattern(&data, pattern), 1);
    }

    #[test]
    fn test_count_chars_empty() {
        assert_eq!(count_chars(b""), 0);
    }

    #[test]
    fn test_count_chars_ascii() {
        assert_eq!(count_chars(b"hello world"), 11);
    }

    #[test]
    fn test_count_chars_utf8() {
        assert_eq!(count_chars("hello 世界".as_bytes()), 8);
        assert_eq!(count_chars("🦀 Rust".as_bytes()), 6);
    }

    #[test]
    fn test_count_chars_vs_bytes() {
        let text = "café";
        assert_eq!(count_chars(text.as_bytes()), 4);
        assert_eq!(text.as_bytes().len(), 5);
    }

    #[test]
    fn test_max_line_length_empty() {
        assert_eq!(max_line_length(b""), 0);
    }

    #[test]
    fn test_max_line_length_single_line() {
        assert_eq!(max_line_length(b"hello"), 5);
    }

    #[test]
    fn test_max_line_length_multiple_lines() {
        assert_eq!(max_line_length(b"hi\nhello\nbye"), 5);
    }

    #[test]
    fn test_max_line_length_trailing_newline() {
        assert_eq!(max_line_length(b"hello\nworld\n"), 5);
    }

    #[test]
    fn test_max_line_length_empty_lines() {
        assert_eq!(max_line_length(b"\n\nhello\n\n"), 5);
    }

    #[test]
    fn test_max_line_length_crlf() {
        assert_eq!(max_line_length(b"hello\r\nworld\r\n"), 5);
        assert_eq!(max_line_length(b"hi\r\nhello\r\nbye\r\n"), 5);
    }

    #[test]
    fn test_max_line_length_mixed_endings() {
        assert_eq!(max_line_length(b"hello\nworld\r\nfoo\n"), 5);
    }

    #[test]
    fn test_filter_code_c_style_single_line() {
        let input = b"// this is a comment\nint x = 5;\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"int x = 5;\n");
    }

    #[test]
    fn test_filter_code_c_style_multiline() {
        let input = b"/* multiline\ncomment */\nint x = 5;\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"int x = 5;\n");
    }

    #[test]
    fn test_filter_code_hash_comments() {
        let input = b"# Python comment\nprint('hello')\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"print('hello')\n");
    }

    #[test]
    fn test_filter_code_sql_comments() {
        let input = b"-- SQL comment\nSELECT * FROM users;\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"SELECT * FROM users;\n");
    }

    #[test]
    fn test_filter_code_python_docstring() {
        let input = b"\"\"\"\nThis is a docstring\n\"\"\"\ndef foo():\n    pass\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"def foo():\n    pass\n");
    }

    #[test]
    fn test_filter_code_empty_lines() {
        let input = b"int x = 5;\n\nint y = 10;\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"int x = 5;\nint y = 10;\n");
    }

    #[test]
    fn test_filter_code_preserves_urls_and_colors() {
        let input = b"url = \"https://example.com#anchor\"\ncolor = \"#fff\"\n";
        let output = filter_code_comments(input);
        assert!(String::from_utf8_lossy(&output).contains("#anchor"));
        assert!(String::from_utf8_lossy(&output).contains("#fff"));
    }

    #[test]
    fn test_filter_code_preserves_sql_operators() {
        let input = b"SELECT * FROM foo--bar WHERE x = 1\n";
        let output = filter_code_comments(input);
        assert!(String::from_utf8_lossy(&output).contains("foo--bar"));
    }

    #[test]
    fn test_filter_markdown_code_block() {
        let input = b"Some text\n```rust\nlet x = 5;\n```\nMore text\n";
        let output = filter_markdown_code(input);
        assert!(String::from_utf8_lossy(&output).contains("Some text"));
        assert!(String::from_utf8_lossy(&output).contains("More text"));
        assert!(!String::from_utf8_lossy(&output).contains("let x = 5"));
    }

    #[test]
    fn test_filter_markdown_inline_code() {
        let input = b"Use the `println!` macro\n";
        let output = filter_markdown_code(input);
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("Use the"));
        assert!(output_str.contains("macro"));
        assert!(!output_str.contains("println!"));
    }

    #[test]
    fn test_filter_markdown_multiple_blocks() {
        let input = b"Intro\n```\ncode1\n```\nMiddle\n```\ncode2\n```\nEnd\n";
        let output = filter_markdown_code(input);
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("Intro"));
        assert!(output_str.contains("Middle"));
        assert!(output_str.contains("End"));
        assert!(!output_str.contains("code1"));
        assert!(!output_str.contains("code2"));
    }

    #[test]
    fn test_unique_words_basic() {
        let input = b"hello world hello foo world bar";
        assert_eq!(count_unique_words(input), 4);
    }

    #[test]
    fn test_unique_words_empty() {
        assert_eq!(count_unique_words(b""), 0);
    }

    #[test]
    fn test_unique_words_all_same() {
        let input = b"word word word word word";
        assert_eq!(count_unique_words(input), 1);
    }

    #[test]
    fn test_utf8_boundary_detection() {
        let text = "hello 世界 test";
        let data = text.as_bytes();

        let boundary = find_utf8_boundary(data, 7);
        assert!(std::str::from_utf8(&data[boundary..]).is_ok());
    }

    #[test]
    fn test_decode_utf8_passthrough() {
        let input = "hello world".as_bytes();
        let output = decode_to_utf8(input, Some("utf-8"));
        assert_eq!(output, input);
    }

    #[test]
    fn test_decode_autodetect_utf8() {
        let input = "hello 世界".as_bytes();
        let output = decode_to_utf8(input, None);
        assert_eq!(output, input);
    }
}
