use clap::Parser;
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(version, about = "Fast wc replacement", long_about = None)]
pub struct Args {
    #[arg(help = "Files to process (reads from stdin if not provided)")]
    pub files: Vec<String>,

    #[arg(short = 'l', long = "lines", help = "Print line counts")]
    pub lines: bool,

    #[arg(short = 'c', long = "bytes", help = "Print byte counts")]
    pub bytes: bool,

    #[arg(short = 'm', long = "chars", help = "Print character counts")]
    pub chars: bool,

    #[arg(short = 'w', long = "words", help = "Print word counts")]
    pub words: bool,

    #[arg(
        short = 'L',
        long = "max-line-length",
        help = "Print length of longest line"
    )]
    pub max_line_length: bool,

    #[arg(long = "pattern", help = "Count occurrences of a specific pattern")]
    pub pattern: Option<String>,

    #[arg(
        long = "files0-from",
        value_name = "FILE",
        help = "Read null-terminated file names from FILE"
    )]
    pub files0_from: Option<String>,

    #[arg(
        long = "generate-completion",
        value_name = "SHELL",
        help = "Generate shell completion script"
    )]
    pub generate_completion: Option<Shell>,

    #[arg(long = "json", help = "Output results as JSON")]
    pub json: bool,

    #[arg(long = "stats", help = "Show detailed statistics")]
    pub stats: bool,

    #[arg(long = "unique", help = "Count unique words")]
    pub unique: bool,

    #[arg(
        short = 'r',
        long = "recursive",
        help = "Process directories recursively"
    )]
    pub recursive: bool,

    #[arg(
        long = "exclude",
        help = "Exclude files matching pattern (can be used multiple times)"
    )]
    pub exclude: Vec<String>,

    #[arg(long = "fast", help = "Skip UTF-8 validation for faster processing")]
    pub fast: bool,

    #[arg(long = "histogram", help = "Show line length histogram")]
    pub histogram: bool,

    #[arg(
        long = "code",
        help = "Count only code (skip comments and blank lines)"
    )]
    pub code: bool,

    #[arg(long = "markdown", help = "Count markdown text (skip code blocks)")]
    pub markdown: bool,

    #[arg(short = 'v', long = "verbose", help = "Show warnings and errors")]
    pub verbose: bool,

    #[arg(long = "timing", help = "Show processing time for each file")]
    pub timing: bool,

    #[arg(short = 'b', long = "blank-lines", help = "Print blank line counts")]
    pub blank_lines: bool,

    #[arg(long = "total-only", help = "Only show total, skip per-file output")]
    pub total_only: bool,

    #[arg(
        long = "encoding",
        value_name = "ENCODING",
        help = "Force input encoding (e.g., utf-8, iso-8859-1, shift_jis). Auto-detects if not specified"
    )]
    pub encoding: Option<String>,

    #[arg(long = "progress", help = "Show progress while processing files")]
    pub progress: bool,
}

impl Args {
    pub fn normalize(&mut self) {
        if !self.lines
            && !self.bytes
            && !self.chars
            && !self.words
            && !self.max_line_length
            && self.pattern.is_none()
            && !self.stats
            && !self.unique
            && !self.histogram
            && !self.blank_lines
        {
            self.lines = true;
            self.bytes = true;
            self.words = true;
        }
    }
}
