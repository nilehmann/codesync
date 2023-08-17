use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    error::Error,
    io,
    ops::Range,
    path::{Path, PathBuf},
};

use clap::Parser;
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use codesync::{Comment, Matches, ParseError};

#[derive(Parser)]
enum Args {
    Check,
}

type FileId = usize;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let matches = Matches::collect()?;
    match args {
        Args::Check => {
            Checker::new().check(&matches)?;
        }
    }

    Ok(())
}

struct Checker {
    db: FilesDB,
    has_errors: bool,
    writer: StandardStream,
    config: codespan_reporting::term::Config,
}

impl Checker {
    fn new() -> Self {
        Self {
            db: FilesDB::new(),
            has_errors: false,
            writer: StandardStream::stderr(ColorChoice::Always),
            config: codespan_reporting::term::Config::default(),
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
                let message =
                    format!("all comments with label `{label}` must have the same count",);
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
        let labels = comments
            .iter()
            .map(|comment| self.db.primary_label(comment.file(), comment.span()))
            .collect::<io::Result<_>>()?;
        Ok(Diagnostic::error()
            .with_message(message)
            .with_labels(labels))
    }

    fn malformed_diagnostic(
        &mut self,
        path: &Path,
        span: Range<usize>,
    ) -> io::Result<Diagnostic<FileId>> {
        let label = self.db.primary_label(path, span)?;
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
        let label = self.db.primary_label(path, span)?;
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
        term::emit(
            &mut self.writer.lock(),
            &self.config,
            &self.db.files,
            &diagnostic,
        )
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

    fn primary_label(&mut self, path: &Path, span: Range<usize>) -> io::Result<Label<FileId>> {
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
