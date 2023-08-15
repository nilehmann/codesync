use std::{error::Error, ops::Range};

use clap::Parser;
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFile,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use codesync::{Matches, ParseError};

#[derive(Parser)]
enum Args {
    Check,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let matches = Matches::collect()?;
    match args {
        Args::Check => check(&matches)?,
    }

    Ok(())
}

fn check(matches: &Matches) -> Result<(), Box<dyn Error>> {
    if check_invalid(&matches)? {
        std::process::exit(1);
    }

    Ok(())
}

fn check_invalid(matches: &Matches) -> Result<bool, Box<dyn Error>> {
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    let mut has_invalid = false;
    for file in &matches.files {
        let simple_file = SimpleFile::new(file.path.display().to_string(), file.content()?);
        for m in file.invalid() {
            let diagnostic = match m.error {
                ParseError::Malformed => malformed_diagnostic(m.span()),
                ParseError::InvalidCount { start, end } => invalid_count_diagnostic(start..end),
            };
            term::emit(&mut writer.lock(), &config, &simple_file, &diagnostic)?;
            has_invalid = true;
        }
    }
    Ok(has_invalid)
}

fn malformed_diagnostic(span: Range<usize>) -> Diagnostic<()> {
    let note = "comment must contain a label and an optional count, e.g., `CODESYNC(my-label)`, `CODESYNC(my-label, 3)`".to_string();
    Diagnostic::error()
        .with_message("malformed codesync comment")
        .with_labels(vec![Label::primary((), span)])
        .with_notes(vec![note])
}

fn invalid_count_diagnostic(span: Range<usize>) -> Diagnostic<()> {
    Diagnostic::error()
        .with_message("invalid count")
        .with_labels(vec![Label::primary((), span)])
        .with_notes(vec!["second argument must be an integer".to_string()])
}
