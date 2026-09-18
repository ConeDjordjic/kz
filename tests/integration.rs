use std::fs::{self, File};
use std::io::Write;
use std::process::Command;

fn kz_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kz"))
}

fn create_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

mod basic_counting {
    use super::*;

    #[test]
    fn count_lines_single_file() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "line1\nline2\nline3\n").unwrap();

        let output = kz_cmd().arg("-l").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("3"));
    }

    #[test]
    fn count_words_single_file() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world foo bar\n").unwrap();

        let output = kz_cmd().arg("-w").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("4"));
    }

    #[test]
    fn count_bytes_single_file() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let output = kz_cmd().arg("-c").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("5"));
    }

    #[test]
    fn count_chars_utf8() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello 世界").unwrap();

        let output = kz_cmd().arg("-m").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("8"));
    }

    #[test]
    fn default_output_lwc() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world\nfoo bar\n").unwrap();

        let output = kz_cmd().arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("2"));
        assert!(stdout.contains("4"));
        assert!(stdout.contains("20"));
    }
}

mod multi_file {
    use super::*;

    #[test]
    fn multiple_files_with_total() {
        let dir = create_temp_dir();
        let file1 = dir.path().join("a.txt");
        let file2 = dir.path().join("b.txt");
        fs::write(&file1, "line1\nline2\n").unwrap();
        fs::write(&file2, "line3\nline4\nline5\n").unwrap();

        let output = kz_cmd().arg("-l").arg(&file1).arg(&file2).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("total"));
        assert!(stdout.contains("5"));
    }

    #[test]
    fn total_only_flag() {
        let dir = create_temp_dir();
        let file1 = dir.path().join("a.txt");
        let file2 = dir.path().join("b.txt");
        fs::write(&file1, "aa\n").unwrap();
        fs::write(&file2, "bb\n").unwrap();

        let output = kz_cmd()
            .arg("-l")
            .arg("--total-only")
            .arg(&file1)
            .arg(&file2)
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.trim().lines().collect();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("total"));
    }
}

mod recursive {
    use super::*;

    #[test]
    fn recursive_directory() {
        let dir = create_temp_dir();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        fs::write(dir.path().join("a.txt"), "line1\n").unwrap();
        fs::write(subdir.join("b.txt"), "line2\nline3\n").unwrap();

        let output = kz_cmd()
            .arg("-l")
            .arg("-r")
            .arg(dir.path())
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("total"));
        assert!(stdout.contains("3"));
    }

    #[test]
    fn directory_without_recursive_flag_errors() {
        let dir = create_temp_dir();

        let output = kz_cmd().arg("-l").arg(dir.path()).output().unwrap();

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("directory") || stderr.contains("-r"));
    }

    #[test]
    fn exclude_pattern() {
        let dir = create_temp_dir();
        fs::write(dir.path().join("a.txt"), "line1\n").unwrap();
        fs::write(dir.path().join("b.log"), "line2\nline3\n").unwrap();

        let output = kz_cmd()
            .arg("-l")
            .arg("-r")
            .arg("--exclude")
            .arg("*.log")
            .arg(dir.path())
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("1"));
        assert!(!stdout.contains("b.log"));
    }
}

mod json_output {
    use super::*;

    #[test]
    fn json_single_file() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world\n").unwrap();

        let output = kz_cmd().arg("--json").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(json.is_array());
        let arr = json.as_array().unwrap();
        assert!(!arr.is_empty());
        let first = &arr[0];
        assert!(first.get("counts").is_some());
        assert!(first.get("counts").unwrap().get("lines").is_some());
    }

    #[test]
    fn json_multiple_files() {
        let dir = create_temp_dir();
        let file1 = dir.path().join("a.txt");
        let file2 = dir.path().join("b.txt");
        fs::write(&file1, "hello\n").unwrap();
        fs::write(&file2, "world\n").unwrap();

        let output = kz_cmd()
            .arg("--json")
            .arg(&file1)
            .arg(&file2)
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(json.is_array());
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert!(arr.last().unwrap().get("file").unwrap().as_str().unwrap() == "total");
    }

    #[test]
    fn json_with_stats() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "short\nlonger line here\n").unwrap();

        let output = kz_cmd()
            .arg("--json")
            .arg("--stats")
            .arg(&file)
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(json.is_array());
        let arr = json.as_array().unwrap();
        assert!(!arr.is_empty());
        let first = &arr[0];
        assert!(first.get("counts").unwrap().get("statistics").is_some());
    }
}

mod special_cases {
    use super::*;

    #[test]
    fn empty_file() {
        let dir = create_temp_dir();
        let file = dir.path().join("empty.txt");
        fs::write(&file, "").unwrap();

        let output = kz_cmd().arg("-lwc").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("0"));
    }

    #[test]
    fn file_without_trailing_newline() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "no newline at end").unwrap();

        let output = kz_cmd().arg("-l").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("0"));
    }

    #[test]
    fn crlf_line_endings() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "line1\r\nline2\r\n").unwrap();

        let output = kz_cmd().arg("-L").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("5"));
    }

    #[test]
    fn nonexistent_file_errors() {
        let output = kz_cmd().arg("/nonexistent/path/file.txt").output().unwrap();

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("No such file") || stderr.contains("not found"));
    }

    #[test]
    fn blank_lines_count() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "line1\n\n  \nline2\n\t\n").unwrap();

        let output = kz_cmd().arg("-b").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("3"));
    }

    #[test]
    fn max_line_length() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "short\nthis is a longer line\nmed\n").unwrap();

        let output = kz_cmd().arg("-L").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("21"));
    }
}

mod pattern_matching {
    use super::*;

    #[test]
    fn pattern_count() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "foo bar foo baz foo\n").unwrap();

        let output = kz_cmd()
            .arg("--pattern")
            .arg("foo")
            .arg(&file)
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("3"));
    }

    #[test]
    fn pattern_no_matches() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world\n").unwrap();

        let output = kz_cmd()
            .arg("--pattern")
            .arg("xyz")
            .arg(&file)
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("0"));
    }
}

mod unique_words {
    use super::*;

    #[test]
    fn unique_word_count() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world hello foo world bar\n").unwrap();

        let output = kz_cmd().arg("--unique").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("4"));
    }

    #[test]
    fn unique_words_large_file() {
        let dir = create_temp_dir();
        let file = dir.path().join("large.txt");
        let content = "word1 word2 word3 word1 word2\n".repeat(50000);
        fs::write(&file, content).unwrap();

        let output = kz_cmd().arg("--unique").arg(&file).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("3"));
    }
}

mod files_from {
    use super::*;

    #[test]
    fn files0_from_file() {
        let dir = create_temp_dir();
        let file1 = dir.path().join("a.txt");
        let file2 = dir.path().join("b.txt");
        let list_file = dir.path().join("files.txt");

        fs::write(&file1, "line1\n").unwrap();
        fs::write(&file2, "line2\nline3\n").unwrap();

        let mut list = File::create(&list_file).unwrap();
        write!(list, "{}\0{}\0", file1.display(), file2.display()).unwrap();

        let output = kz_cmd()
            .arg("-l")
            .arg("--files0-from")
            .arg(&list_file)
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("total"));
        assert!(stdout.contains("3"));
    }
}

mod filtering {
    use super::*;

    #[test]
    fn filter_code_comments() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.rs");
        fs::write(&file, "// comment\nlet x = 5;\nlet y = 10;\n").unwrap();

        let output = kz_cmd()
            .arg("-w")
            .arg("--code")
            .arg(&file)
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("8"));
    }

    #[test]
    fn filter_markdown_code() {
        let dir = create_temp_dir();
        let file = dir.path().join("test.md");
        fs::write(&file, "Some text\n```\ncode here\n```\nMore text\n").unwrap();

        let output = kz_cmd()
            .arg("-w")
            .arg("--markdown")
            .arg(&file)
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("4"));
    }
}
