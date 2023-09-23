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
use codesync::{inflector, Arg, ArgsError, Comment, Matches};
use regex::Regex;

#[derive(Parser)]
#[command(disable_help_subcommand = true)]
enum Args {
    /// Check that all CODESYNC matches are well-formed and their counts are correct.
    Check(CheckArgs),
    /// Show all valid CODESYNC comments with a given label. This ignores invalid matches.
    Show { label: String },
    /// List all labels from valid comments. This ignores invalid matches.
    List,
}

#[derive(clap::Args)]
struct CheckArgs {
    /// Check that all labels use the same casing.
    #[arg(long)]
    consistent_casing: Option<Case>,
    /// Check that there is no extra whitespace around arguments.
    #[arg(long)]
    no_extra_whitespace: bool,
    /// Check that labels match the given regex.
    #[arg(long)]
    label_pattern: Option<Regex>,
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum Case {
    #[value(name = "camelCase", aliases(["camel-case", "camel"]))]
    Camel,
    #[value(name = "kebab-case", aliases(["kebab-case", "kebab"]))]
    Kebab,
    #[value(name = "PascalCase", aliases(["pascal-case", "pascal"]))]
    Pascal,
    #[value(name = "SCREAMING_SNAKE_CASE", aliases(["screaming-snake-case", "screaming-snake"]))]
    ScreamingSnake,
    #[value(name = "snake_case", aliases(["snake-case", "snake"]))]
    Snake,
    #[value(name = "Train-Case", aliases(["train-case", "train"]))]
    Train,
}

impl Case {
    fn has_case(self, s: &str) -> bool {
        match self {
            Case::Camel => inflector::is_camel_case(s),
            Case::Kebab => inflector::is_kebab_case(s),
            Case::Pascal => inflector::is_pascal_case(s),
            Case::ScreamingSnake => inflector::is_screaming_snake_case(s),
            Case::Snake => inflector::is_snake_case(s),
            Case::Train => inflector::is_train_case(s),
        }
    }

    fn to_case(self, s: &str) -> String {
        match self {
            Case::Camel => inflector::to_camel_case(s, &HashSet::new()),
            Case::Kebab => inflector::to_kebab_case(s),
            Case::Pascal => inflector::to_pascal_case(s),
            Case::ScreamingSnake => inflector::to_screaming_snake_case(s),
            Case::Snake => inflector::to_snake_case(s),
            Case::Train => inflector::to_train_case(s),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Case::Camel => "camel",
            Case::Kebab => "kebab",
            Case::Pascal => "pascal",
            Case::ScreamingSnake => "screaming snake",
            Case::Snake => "snake",
            Case::Train => "train",
        }
    }
}

impl std::fmt::Display for Case {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
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
        Args::Check(args) => {
            Checker::new(args).check(&matches)?;
        }
        Args::Show { label } => {
            let mut db = FilesDB::new();
            let mut emitter = Emitter::new(false);
            let comments = matches.comments().filter(|c| &c.label() == &label);
            let diagnostic = Diagnostic::note()
                .with_message(format!("showing comments for label `{label}`"))
                .with_labels(db.labels(comments)?);
            emitter.emit(&db, diagnostic)?;
        }
        Args::List => {
            let stdout = &mut StandardStream::stdout(ColorChoice::Auto);
            stdout.set_color(ColorSpec::new().set_bold(true))?;
            for (label, _) in matches.group_by_label() {
                write!(stdout, "{label}\n")?;
            }
            stdout.reset()?;
            writeln!(stdout)?;
        }
    }

    Ok(())
}

struct Emitter {
    writer: StandardStream,
    config: codespan_reporting::term::Config,
    has_errors: bool,
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
            has_errors: false,
        }
    }

    fn abort_if_errors(&self) {
        if self.has_errors {
            std::process::exit(1);
        }
    }

    fn emit(
        &mut self,
        db: &FilesDB,
        diagnostic: Diagnostic<FileId>,
    ) -> Result<(), codespan_reporting::files::Error> {
        self.has_errors = true;
        term::emit(
            &mut self.writer.lock(),
            &self.config,
            &db.files,
            &diagnostic,
        )
    }
}

struct Checker {
    args: CheckArgs,
    db: FilesDB,
    emitter: Emitter,
}

impl Checker {
    fn new(args: CheckArgs) -> Self {
        Self {
            args,
            db: FilesDB::new(),
            emitter: Emitter::new(true),
        }
    }

    fn check(&mut self, matches: &Matches) -> Result<(), Box<dyn Error>> {
        self.report_invalid_matches(&matches)?;
        self.abort_if_errors();

        for (label, comments) in matches.group_by_label() {
            self.report_incorrect_counts(label, &comments)?;
        }
        self.abort_if_errors();

        self.report_inconsistent_casing(matches)?;
        self.abort_if_errors();

        self.report_label_regex_mismatch(matches)?;

        self.report_no_extra_whitespace(matches)?;
        self.abort_if_errors();

        Ok(())
    }

    fn report_invalid_matches(&mut self, matches: &Matches) -> Result<(), Box<dyn Error>> {
        for m in matches.invalid_matches() {
            let diagnostic = match m.error {
                ArgsError::Malformed => self.db.malformed_diagnostic(m.file(), m.span())?,
                ArgsError::InvalidCount { start, end } => {
                    self.db.invalid_count_diagnostic(m.file(), start..end)?
                }
            };
            self.emit_diagnostic(diagnostic)?;
        }
        Ok(())
    }

    fn report_incorrect_counts(
        &mut self,
        label: &str,
        comments: &[Comment],
    ) -> Result<(), Box<dyn Error>> {
        let counts: Vec<_> = comments
            .iter()
            .map(Comment::count)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        match &counts[..] {
            [] => {}
            [count] => {
                let expected = *count as usize;
                let found = comments.len();
                if found != expected {
                    let message = format!(
                        "expected {expected} {} with label `{label}`, found {found}",
                        pluralize("comment", expected)
                    );
                    let diagnostic = self.db.mismatched_counts_diagnostic(comments, message)?;
                    self.emit_diagnostic(diagnostic)?;
                }
            }
            _ => {
                let message = format!("not all comments with label `{label}` have the same count",);
                let diagnostic = self.db.mismatched_counts_diagnostic(comments, message)?;
                self.emit_diagnostic(diagnostic)?;
            }
        }

        Ok(())
    }

    fn report_inconsistent_casing(&mut self, matches: &Matches) -> Result<(), Box<dyn Error>> {
        if let Some(case) = self.args.consistent_casing {
            for comment in matches.comments() {
                if !case.has_case(comment.label()) {
                    let diagnostic = self.db.invalid_case_diagnostic(comment, case)?;
                    self.emit_diagnostic(diagnostic)?;
                }
            }
        }
        Ok(())
    }

    fn report_no_extra_whitespace(&mut self, matches: &Matches) -> Result<(), Box<dyn Error>> {
        if self.args.no_extra_whitespace {
            for comment in matches.comments() {
                if let Some(count_arg) = comment.count_arg() {
                    if count_arg.has_extra_whitespace() {
                        let diagnostic = self
                            .db
                            .extra_whitespace_diagnostic(comment.file(), count_arg)?;
                        self.emit_diagnostic(diagnostic)?;
                    }
                }
                let label_arg = comment.label_arg();
                if label_arg.has_extra_whitespace() {
                    if label_arg.has_extra_whitespace() {
                        let diagnostic = self
                            .db
                            .extra_whitespace_diagnostic(comment.file(), label_arg)?;
                        self.emit_diagnostic(diagnostic)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn report_label_regex_mismatch(&mut self, matches: &Matches) -> Result<(), Box<dyn Error>> {
        if let Some(re) = &self.args.label_pattern {
            for comment in matches.comments() {
                if !re.is_match(comment.label()) {
                    let diagnostic = self.db.regex_mismatch_diagnostic(comment)?;
                    self.emitter.emit(&self.db, diagnostic)?;
                }
            }
        }
        Ok(())
    }

    fn abort_if_errors(&self) {
        self.emitter.abort_if_errors();
    }

    fn emit_diagnostic(
        &mut self,
        diagnostic: Diagnostic<FileId>,
    ) -> Result<(), codespan_reporting::files::Error> {
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

    fn regex_mismatch_diagnostic(&mut self, comment: Comment) -> io::Result<Diagnostic<FileId>> {
        let label = self.label(comment.file(), comment.label_arg().span())?;
        Ok(Diagnostic::error()
            .with_message("label doesn't match regex")
            .with_labels(vec![label]))
    }

    fn extra_whitespace_diagnostic<T>(
        &mut self,
        file: &Path,
        arg: &Arg<T>,
    ) -> io::Result<Diagnostic<FileId>> {
        let label = self.label(file, arg.span())?;
        Ok(Diagnostic::error()
            .with_message("argument has extra whitespace")
            .with_labels(vec![label]))
    }

    fn invalid_case_diagnostic(
        &mut self,
        comment: Comment,
        case: Case,
    ) -> io::Result<Diagnostic<FileId>> {
        let label = self
            .label(comment.file(), comment.span())?
            .with_message(format!(
                "should be written as {}",
                case.to_case(comment.label())
            ));
        Ok(Diagnostic::error()
            .with_message(format!("label doesn't use {case} case"))
            .with_labels(vec![label]))
    }

    fn mismatched_counts_diagnostic(
        &mut self,
        comments: &[Comment],
        message: impl Into<String>,
    ) -> io::Result<Diagnostic<FileId>> {
        let labels = self.labels(comments.iter().copied())?;
        Ok(Diagnostic::error()
            .with_message(message)
            .with_labels(labels))
    }

    fn malformed_diagnostic(
        &mut self,
        path: &Path,
        span: Range<usize>,
    ) -> io::Result<Diagnostic<FileId>> {
        let label = self.label(path, span)?;
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
        let label = self.label(path, span)?;
        Ok(Diagnostic::error()
            .with_message("invalid count")
            .with_labels(vec![label])
            .with_notes(vec!["second argument must be an integer".to_string()]))
    }
}

fn pluralize(word: &str, count: usize) -> String {
    if count == 1 {
        word.to_string()
    } else {
        format!("{word}s")
    }
}
