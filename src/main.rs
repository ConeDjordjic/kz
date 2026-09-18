mod config;
mod count;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use encoding_rs::Encoding;
use globset::{Glob, GlobSetBuilder};
use memmap2::MmapOptions;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use walkdir::WalkDir;

const MAX_WALKDIR_DEPTH: usize = 100;

#[derive(Serialize)]
struct Counts {
    lines: usize,
    words: usize,
    bytes: usize,
    chars: usize,
    max_line_length: usize,
    blank_lines: usize,
    pattern: usize,
    unique_words: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    statistics: Option<Statistics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    histogram: Option<HashMap<usize, usize>>,
}

#[derive(Serialize)]
struct Statistics {
    mean_line_length: f64,
    median_line_length: usize,
    std_dev: f64,
    min_line_length: usize,
    max_line_length: usize,
    empty_lines: usize,
}

impl Counts {
    fn new() -> Self {
        Self {
            lines: 0,
            words: 0,
            bytes: 0,
            chars: 0,
            max_line_length: 0,
            blank_lines: 0,
            pattern: 0,
            unique_words: 0,
            statistics: None,
            histogram: None,
        }
    }

    fn add(&mut self, other: &Counts) {
        self.lines += other.lines;
        self.words += other.words;
        self.bytes += other.bytes;
        self.chars += other.chars;
        self.max_line_length = self.max_line_length.max(other.max_line_length);
        self.blank_lines += other.blank_lines;
        self.pattern += other.pattern;
        self.unique_words += other.unique_words;
    }

    fn get_values(&self, args: &config::Args) -> Vec<usize> {
        let mut values = Vec::new();
        if args.lines {
            values.push(self.lines);
        }
        if args.words {
            values.push(self.words);
        }
        if args.chars {
            values.push(self.chars);
        }
        if args.bytes {
            values.push(self.bytes);
        }
        if args.max_line_length {
            values.push(self.max_line_length);
        }
        if args.blank_lines {
            values.push(self.blank_lines);
        }
        if args.unique {
            values.push(self.unique_words);
        }
        if args.pattern.is_some() {
            values.push(self.pattern);
        }
        values
    }

    fn format(&self, args: &config::Args, name: &str, widths: &[usize]) -> String {
        let values = self.get_values(args);

        let formatted: Vec<String> = values
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{:>width$}", v, width = widths.get(i).copied().unwrap_or(1)))
            .collect();

        if name.is_empty() {
            formatted.join(" ")
        } else {
            format!("{} {}", formatted.join(" "), name)
        }
    }

    fn format_stats(&self) -> String {
        if let Some(ref stats) = self.statistics {
            format!(
                "Statistics:\n  Lines: {}\n  Words: {}\n  Bytes: {}\n  Mean line length: {:.2}\n  Median line length: {}\n  Std deviation: {:.2}\n  Min line length: {}\n  Max line length: {}\n  Empty lines: {}",
                self.lines,
                self.words,
                self.bytes,
                stats.mean_line_length,
                stats.median_line_length,
                stats.std_dev,
                stats.min_line_length,
                stats.max_line_length,
                stats.empty_lines
            )
        } else {
            String::new()
        }
    }

    fn format_histogram(&self) -> String {
        if let Some(ref hist) = self.histogram {
            let mut sorted: Vec<_> = hist.iter().collect();
            sorted.sort_by_key(|(k, _)| **k);

            let max_count = *hist.values().max().unwrap_or(&1);
            let max_bar_width = 50;

            let mut result = String::from("Line Length Histogram:\n");
            for (bucket, count) in sorted {
                let bar_width =
                    ((*count as f64 / max_count as f64) * max_bar_width as f64) as usize;
                let bar = "█".repeat(bar_width);
                result.push_str(&format!(
                    "  {:4}-{:4}: {:6} {}\n",
                    bucket,
                    bucket + 9,
                    count,
                    bar
                ));
            }
            result
        } else {
            String::new()
        }
    }
}

fn process_data(data: &[u8], args: &config::Args) -> Counts {
    let mut counts = Counts::new();

    let needs_decoding = args.encoding.is_some()
        || args.words
        || args.chars
        || args.unique
        || args.stats
        || args.code
        || args.markdown;

    let decoded_data;
    let data_after_encoding = if needs_decoding {
        let validated_encoding = args.encoding.as_deref().and_then(|name| {
            if Encoding::for_label(name.as_bytes()).is_some() {
                Some(name)
            } else {
                if args.verbose {
                    eprintln!(
                        "kz: warning: unknown encoding '{}', falling back to auto-detection",
                        name
                    );
                }
                None
            }
        });
        decoded_data = count::decode_to_utf8(data, validated_encoding);
        &decoded_data[..]
    } else {
        data
    };

    let filtered_data;
    let data_to_process = if args.code {
        filtered_data = count::filter_code_comments(data_after_encoding);
        &filtered_data
    } else if args.markdown {
        filtered_data = count::filter_markdown_code(data_after_encoding);
        &filtered_data
    } else {
        data_after_encoding
    };

    if args.lines || args.stats {
        counts.lines = count::count_lines(data_to_process);
    }
    if args.words || args.stats {
        counts.words = count::count_all_words(data_to_process);
    }
    if args.chars {
        if args.fast {
            counts.chars = data_to_process.len();
        } else {
            counts.chars = count::count_chars(data_to_process);
        }
    }
    if args.bytes || args.stats {
        counts.bytes = data_to_process.len();
    }
    if args.max_line_length {
        counts.max_line_length = count::max_line_length(data_to_process);
    }
    if args.blank_lines {
        counts.blank_lines = count::count_blank_lines(data_to_process);
    }
    if args.unique {
        counts.unique_words = count::count_unique_words(data_to_process);
    }
    if let Some(pattern) = &args.pattern {
        counts.pattern = count::count_pattern(data_to_process, pattern.as_bytes());
    }
    if args.stats {
        let stats = count::calculate_statistics(data_to_process);
        counts.statistics = Some(Statistics {
            mean_line_length: stats.mean_line_length,
            median_line_length: stats.median_line_length,
            std_dev: stats.std_dev,
            min_line_length: stats.min_line_length,
            max_line_length: stats.max_line_length,
            empty_lines: stats.empty_lines,
        });
    }
    if args.histogram {
        counts.histogram = Some(count::generate_histogram(data_to_process));
    }

    counts
}

struct FileResult {
    counts: Counts,
    duration: Option<std::time::Duration>,
}

fn process_file(path: &str, args: &config::Args) -> io::Result<FileResult> {
    let start = if args.timing {
        Some(Instant::now())
    } else {
        None
    };

    let needs_only_bytes = args.bytes
        && !args.lines
        && !args.words
        && !args.chars
        && !args.max_line_length
        && !args.blank_lines
        && !args.unique
        && args.pattern.is_none()
        && !args.stats
        && !args.histogram
        && !args.code
        && !args.markdown
        && args.encoding.is_none();

    if needs_only_bytes {
        let metadata = std::fs::metadata(path)?;
        let mut counts = Counts::new();
        counts.bytes = metadata.len() as usize;
        return Ok(FileResult {
            counts,
            duration: start.map(|s| s.elapsed()),
        });
    }

    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len() as usize;

    if file_size == 0 {
        return Ok(FileResult {
            counts: Counts::new(),
            duration: start.map(|s| s.elapsed()),
        });
    }

    const MMAP_THRESHOLD: usize = 128 * 1024;

    let counts = if file_size >= MMAP_THRESHOLD && metadata.is_file() {
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        if count::is_binary(&mmap) {
            eprintln!("kz: {}: binary file detected, skipping", path);
            return Ok(FileResult {
                counts: Counts::new(),
                duration: start.map(|s| s.elapsed()),
            });
        }

        process_data(&mmap, args)
    } else {
        let mut buffer = Vec::with_capacity(file_size);
        let mut file = file;
        file.read_to_end(&mut buffer)?;

        if count::is_binary(&buffer) {
            eprintln!("kz: {}: binary file detected, skipping", path);
            return Ok(FileResult {
                counts: Counts::new(),
                duration: start.map(|s| s.elapsed()),
            });
        }

        process_data(&buffer, args)
    };

    Ok(FileResult {
        counts,
        duration: start.map(|s| s.elapsed()),
    })
}

fn process_stdin(args: &config::Args) -> io::Result<FileResult> {
    let start = if args.timing {
        Some(Instant::now())
    } else {
        None
    };

    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;

    if count::is_binary(&buffer) {
        eprintln!("kz: stdin: binary data detected, skipping");
        return Ok(FileResult {
            counts: Counts::new(),
            duration: start.map(|s| s.elapsed()),
        });
    }

    Ok(FileResult {
        counts: process_data(&buffer, args),
        duration: start.map(|s| s.elapsed()),
    })
}

fn read_files_from_file(path: &str) -> io::Result<Vec<String>> {
    let mut content = Vec::new();
    if path == "-" {
        io::stdin().read_to_end(&mut content)?;
    } else {
        let mut file = File::open(path)?;
        file.read_to_end(&mut content)?;
    }

    Ok(content
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .filter_map(|s| std::str::from_utf8(s).ok())
        .map(|s| s.to_string())
        .collect())
}

fn collect_files(args: &config::Args) -> io::Result<Vec<String>> {
    let mut all_files = Vec::new();

    let mut exclude_builder = GlobSetBuilder::new();
    for pattern in &args.exclude {
        let glob =
            Glob::new(pattern).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        exclude_builder.add(glob);
    }
    let exclude_set = exclude_builder
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    if let Some(ref files0_path) = args.files0_from {
        let files = read_files_from_file(files0_path)?;
        all_files.extend(files);
    }

    for path_str in &args.files {
        let path = Path::new(path_str);

        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{}: No such file or directory", path_str),
            ));
        }

        if path.is_file() {
            all_files.push(path_str.clone());
        } else if path.is_dir() {
            if !args.recursive {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{}: Is a directory (use -r for recursive)", path_str),
                ));
            }

            for entry in WalkDir::new(path)
                .follow_links(true)
                .max_depth(MAX_WALKDIR_DEPTH)
            {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        if args.verbose {
                            eprintln!("kz: warning: {}", e);
                        }
                        continue;
                    }
                };
                let entry_path = entry.path();

                if !entry_path.is_file() {
                    continue;
                }

                if !args.exclude.is_empty() && exclude_set.is_match(entry_path) {
                    continue;
                }

                if let Some(path_str) = entry_path.to_str() {
                    all_files.push(path_str.to_string());
                }
            }
        }
    }

    Ok(all_files)
}

fn main() {
    let mut args = config::Args::parse();

    if let Some(shell) = args.generate_completion {
        let mut cmd = config::Args::command();
        generate(shell, &mut cmd, "kz", &mut io::stdout());
        return;
    }

    args.normalize();

    if args.files.is_empty() && args.files0_from.is_none() {
        if atty::is(atty::Stream::Stdin) {
            eprintln!("kz: no input provided (use --help for usage)");
            std::process::exit(1);
        }

        match process_stdin(&args) {
            Ok(result) => {
                if args.json {
                    let mut json_obj = serde_json::Map::new();
                    if let Ok(counts_value) = serde_json::to_value(&result.counts)
                        && let Some(obj) = counts_value.as_object()
                    {
                        for (k, v) in obj {
                            json_obj.insert(k.clone(), v.clone());
                        }
                    }
                    if let Some(duration) = result.duration {
                        let ms = duration.as_secs_f64() * 1000.0;
                        if let Some(num) = serde_json::Number::from_f64(ms) {
                            json_obj
                                .insert("duration_ms".to_string(), serde_json::Value::Number(num));
                        }
                    }
                    match serde_json::to_string_pretty(&serde_json::Value::Object(json_obj)) {
                        Ok(json) => println!("{}", json),
                        Err(e) => {
                            eprintln!("kz: JSON serialization error: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else if args.stats {
                    let mut output = result.counts.format_stats();
                    if let Some(duration) = result.duration {
                        output.push_str(&format!(
                            "\n  Duration: {:.3}ms",
                            duration.as_secs_f64() * 1000.0
                        ));
                    }
                    println!("{}", output);
                } else if args.histogram {
                    println!("{}", result.counts.format_histogram());
                } else {
                    let widths: Vec<usize> = result
                        .counts
                        .get_values(&args)
                        .iter()
                        .map(|v| v.to_string().len().max(1))
                        .collect();
                    let mut output = result.counts.format(&args, "", &widths);
                    if let Some(duration) = result.duration {
                        output.push_str(&format!(" ({:.3}ms)", duration.as_secs_f64() * 1000.0));
                    }
                    println!("{}", output);
                }
            }
            Err(e) => {
                eprintln!("kz: stdin: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    let files = match collect_files(&args) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("kz: {}", e);
            std::process::exit(1);
        }
    };

    if files.is_empty() {
        eprintln!("kz: no files to process");
        std::process::exit(1);
    }

    let show_total = files.len() > 1;

    let total_start = if args.timing {
        Some(Instant::now())
    } else {
        None
    };

    let file_results: Vec<_> = if files.len() == 1 {
        if args.progress {
            eprint!("\r\x1b[Kprocessing: 1/1 {}", files[0]);
            let _ = io::stderr().flush();
        }
        let results: Vec<_> = files
            .iter()
            .map(|path| (path.clone(), process_file(path, &args)))
            .collect();
        if args.progress {
            eprint!("\r\x1b[K");
            let _ = io::stderr().flush();
        }
        results
    } else {
        let total_files = files.len();
        let processed = AtomicUsize::new(0);
        let progress_lock = Mutex::new(());
        let results: Vec<_> = files
            .par_iter()
            .map(|path| {
                let result = (path.clone(), process_file(path, &args));
                if args.progress {
                    let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    let display_path = if path.len() > 40 {
                        format!("...{}", &path[path.len() - 37..])
                    } else {
                        path.clone()
                    };
                    if let Ok(_guard) = progress_lock.lock() {
                        eprint!(
                            "\r\x1b[Kprocessing: {}/{} {}",
                            count, total_files, display_path
                        );
                        let _ = io::stderr().flush();
                    }
                }
                result
            })
            .collect();
        if args.progress {
            eprint!("\r\x1b[K");
            let _ = io::stderr().flush();
        }
        results
    };

    let total_duration = total_start.map(|s| s.elapsed());

    let mut total = Counts::new();
    let mut had_error = false;
    let mut json_results = Vec::new();

    for (path, result) in &file_results {
        match result {
            Ok(file_result) => {
                total.add(&file_result.counts);
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    if args.verbose {
                        eprintln!("kz: {}: {}", path, e);
                    }
                } else {
                    eprintln!("kz: {}: {}", path, e);
                    had_error = true;
                }
            }
        }
    }

    let widths: Vec<usize> = total
        .get_values(&args)
        .iter()
        .map(|v| v.to_string().len().max(1))
        .collect();

    if !args.total_only {
        for (path, result) in &file_results {
            if let Ok(file_result) = result {
                if args.json {
                    continue;
                } else if args.stats {
                    println!("\n{}", path);
                    let mut output = file_result.counts.format_stats();
                    if let Some(duration) = file_result.duration {
                        output.push_str(&format!(
                            "\n  Duration: {:.3}ms",
                            duration.as_secs_f64() * 1000.0
                        ));
                    }
                    println!("{}", output);
                } else if args.histogram {
                    println!("\n{}", path);
                    println!("{}", file_result.counts.format_histogram());
                } else {
                    let mut output = file_result.counts.format(&args, path, &widths);
                    if let Some(duration) = file_result.duration {
                        output.push_str(&format!(" ({:.3}ms)", duration.as_secs_f64() * 1000.0));
                    }
                    println!("{}", output);
                }
            }
        }
    }

    if args.json {
        if !args.total_only {
            for (path, result) in &file_results {
                if let Ok(file_result) = result {
                    let mut json_obj = serde_json::Map::new();
                    json_obj.insert("file".to_string(), serde_json::Value::String(path.clone()));
                    if let Ok(counts_value) = serde_json::to_value(&file_result.counts) {
                        json_obj.insert("counts".to_string(), counts_value);
                    }
                    if let Some(duration) = file_result.duration {
                        let ms = duration.as_secs_f64() * 1000.0;
                        if let Some(num) = serde_json::Number::from_f64(ms) {
                            json_obj
                                .insert("duration_ms".to_string(), serde_json::Value::Number(num));
                        }
                    }
                    json_results.push(serde_json::Value::Object(json_obj));
                }
            }
        }
        if show_total || args.total_only {
            let mut json_obj = serde_json::Map::new();
            json_obj.insert(
                "file".to_string(),
                serde_json::Value::String("total".to_string()),
            );
            if let Ok(total_value) = serde_json::to_value(&total) {
                json_obj.insert("counts".to_string(), total_value);
            }
            if let Some(duration) = total_duration {
                let ms = duration.as_secs_f64() * 1000.0;
                if let Some(num) = serde_json::Number::from_f64(ms) {
                    json_obj.insert("duration_ms".to_string(), serde_json::Value::Number(num));
                }
            }
            json_results.push(serde_json::Value::Object(json_obj));
        }
        match serde_json::to_string_pretty(&serde_json::Value::Array(json_results)) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("kz: JSON serialization error: {}", e);
                std::process::exit(1);
            }
        }
    } else if (show_total || args.total_only) && !args.stats && !args.histogram {
        let mut output = total.format(&args, "total", &widths);
        if let Some(duration) = total_duration {
            output.push_str(&format!(" ({:.3}ms)", duration.as_secs_f64() * 1000.0));
        }
        println!("{}", output);
    }

    if had_error {
        std::process::exit(1);
    }
}
