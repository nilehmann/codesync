use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    error::Error,
    io::{self, Write},
    ops::Range,
    path::{Path, PathBuf},
};

use clap::Parser;
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor},
    },
};
use codesync::{Comment, Matches, ParseError};

#[derive(Parser)]
#[command(disable_help_subcommand = true)]
/// This utility
enum Args {
    /// Check that all codesync comments are well-formed and their counts are correct.
    Check,
    /// Show all valid codesync comments with a given label.
    Show(ShowArgs),
    /// List all valid labels.
    List,
}

#[derive(clap::Args)]
struct ShowArgs {
    label: String,
}

type FileId = usize;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let matches = Matches::collect()?;
    match args {
        Args::Check => {
            Checker::new().check(&matches)?;
        }
        Args::Show(ShowArgs { label }) => {
            let mut db = FilesDB::new();
            let mut emitter = Emitter::new(false);
            let comments = matches.valid().filter(|m| &m.opts.label == &label);
            let diagnostic = Diagnostic::note()
                .with_message(format!("showing comments for label `{label}`"))
                .with_labels(db.labels(comments)?);
            emitter.emit(&db, diagnostic)?;
        }
        Args::List => {
            let stdout = &mut StandardStream::stdout(ColorChoice::Auto);
            for (label, comments) in matches.group_by_label() {
                stdout.set_color(ColorSpec::new().set_underline(true).set_bold(true))?;
                write!(stdout, "{label}:")?;
                stdout.reset()?;
                writeln!(stdout, " {}", comments.len())?;
            }
            writeln!(stdout)?;
        }
    }

    Ok(())
}

struct Emitter {
    writer: StandardStream,
    config: codespan_reporting::term::Config,
}

impl Emitter {
    fn new(stderr: bool) -> Self {
        let writer = if stderr {
            StandardStream::stderr(ColorChoice::Auto)
        } else {
            StandardStream::stdout(ColorChoice::Auto)
        };
        Self {
            writer,
            config: codespan_reporting::term::Config::default(),
        }
    }

    fn emit(
        &mut self,
        db: &FilesDB,
        diagnostic: Diagnostic<FileId>,
    ) -> Result<(), codespan_reporting::files::Error> {
        term::emit(
            &mut self.writer.lock(),
            &self.config,
            &db.files,
            &diagnostic,
        )
    }
}

struct Checker {
    db: FilesDB,
    has_errors: bool,
    emitter: Emitter,
}

impl Checker {
    fn new() -> Self {
        Self {
            db: FilesDB::new(),
            has_errors: false,
            emitter: Emitter::new(true),
        }
    }

    fn check(&mut self, matches: &Matches) -> Result<(), Box<dyn Error>> {
        self.check_invalid(&matches)?;
        self.abort_if_errors();

        for (label, matches) in matches.group_by_label() {
            self.check_counts(label, &matches)?;
        }
        self.abort_if_errors();

        Ok(())
    }

    fn check_invalid(&mut self, matches: &Matches) -> Result<(), Box<dyn Error>> {
        for m in matches.invalid() {
            let diagnostic = match m.error {
                ParseError::Malformed => self.malformed_diagnostic(m.file(), m.span())?,
                ParseError::InvalidCount { start, end } => {
                    self.invalid_count_diagnostic(m.file(), start..end)?
                }
            };
            self.emit(diagnostic)?;
        }
        Ok(())
    }

    fn check_counts(&mut self, label: &str, matches: &[Comment]) -> Result<(), Box<dyn Error>> {
        let counts: Vec<_> = matches
            .iter()
            .map(|m| m.opts.count())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        match &counts[..] {
            [] => {}
            [count] => {
                let expected = *count as usize;
                let found = matches.len();
                if found != expected {
                    let message = format!(
                        "expected {expected} {} with label `{label}`, found {found}",
                        pluralize("comment", expected)
                    );
                    let diagnostic = self.mismatched_counts_diagnostic(matches, message)?;
                    self.emit(diagnostic)?;
                }
            }
            _ => {
                let message = format!("not all comments with label `{label}` have the same count",);
                let diagnostic = self.mismatched_counts_diagnostic(matches, message)?;
                self.emit(diagnostic)?;
            }
        }

        Ok(())
    }

    fn mismatched_counts_diagnostic(
        &mut self,
        comments: &[Comment],
        message: impl Into<String>,
    ) -> io::Result<Diagnostic<FileId>> {
        let labels = self.db.labels(comments.iter().copied())?;
        Ok(Diagnostic::error()
            .with_message(message)
            .with_labels(labels))
    }

    fn malformed_diagnostic(
        &mut self,
        path: &Path,
        span: Range<usize>,
    ) -> io::Result<Diagnostic<FileId>> {
        let label = self.db.label(path, span)?;
        let note = "comment must contain a label and an optional count, e.g., `CODESYNC(my-label)`, `CODESYNC(my-label, 3)`".to_string();
        Ok(Diagnostic::error()
            .with_message("malformed codesync comment")
            .with_labels(vec![label])
            .with_notes(vec![note]))
    }

    fn invalid_count_diagnostic(
        &mut self,
        path: &Path,
        span: Range<usize>,
    ) -> io::Result<Diagnostic<FileId>> {
        let label = self.db.label(path, span)?;
        Ok(Diagnostic::error()
            .with_message("invalid count")
            .with_labels(vec![label])
            .with_notes(vec!["second argument must be an integer".to_string()]))
    }

    fn abort_if_errors(&self) {
        if self.has_errors {
            std::process::exit(1);
        }
    }

    fn emit(
        &mut self,
        diagnostic: Diagnostic<FileId>,
    ) -> Result<(), codespan_reporting::files::Error> {
        self.has_errors = true;
        self.emitter.emit(&self.db, diagnostic)
    }
}

struct FilesDB {
    pub files: SimpleFiles<String, String>,
    path_to_file_id: HashMap<PathBuf, FileId>,
}

impl FilesDB {
    fn new() -> Self {
        Self {
            files: SimpleFiles::new(),
            path_to_file_id: HashMap::new(),
        }
    }

    fn labels<'a>(
        &mut self,
        comments: impl IntoIterator<Item = Comment<'a>>,
    ) -> io::Result<Vec<Label<FileId>>> {
        comments
            .into_iter()
            .map(|comment| self.label(comment.file(), comment.span()))
            .collect::<io::Result<_>>()
    }

    fn label(&mut self, path: &Path, span: Range<usize>) -> io::Result<Label<FileId>> {
        let file_id = self.try_get_or_insert(path, || std::fs::read_to_string(path))?;
        Ok(Label::primary(file_id, span))
    }

    fn try_get_or_insert<E>(
        &mut self,
        path: &Path,
        f: impl Fn() -> Result<String, E>,
    ) -> Result<FileId, E> {
        match self.path_to_file_id.entry(path.to_path_buf()) {
            Entry::Occupied(entry) => Ok(*entry.get()),
            Entry::Vacant(entry) => {
                let file_id = self.files.add(path.display().to_string(), f()?);
                entry.insert(file_id);
                Ok(file_id)
            }
        }
    }
}

fn pluralize(word: &str, count: usize) -> String {
    if count == 1 {
        word.to_string()
    } else {
        format!("{}s", word)
    }
}
