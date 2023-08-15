use std::{
    cell::OnceCell,
    io,
    ops::Range,
    path::{Path, PathBuf},
    str,
};

mod kmp;

const CODESYNC: &[u8] = b"CODESYNC";
const CODESYNC_KMP_TABLE: [usize; CODESYNC.len()] = kmp::table(CODESYNC);

pub struct Matches {
    pub files: Vec<FileMatches>,
}

pub struct FileMatches {
    pub path: PathBuf,
    matches: Vec<Match>,
    content: OnceCell<String>,
}

impl FileMatches {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            matches: vec![],
            content: OnceCell::new(),
        }
    }

    fn push(&mut self, m: Match) {
        self.matches.push(m)
    }

    pub fn invalid(&self) -> impl Iterator<Item = InvalidMatch> + '_ {
        self.matches.iter().filter_map(Match::to_invalid)
    }

    pub fn content(&self) -> std::io::Result<&str> {
        if let Some(val) = self.content.get() {
            return Ok(val);
        }
        let val = self.read_content()?;
        assert!(self.content.set(val).is_ok(), "reentrant init");
        Ok(self.content.get().unwrap())
    }

    fn read_content(&self) -> std::io::Result<String> {
        std::fs::read_to_string(&self.path)
    }
}

impl Matches {
    pub fn collect() -> Result<Self, ignore::Error> {
        let matcher = Matcher::new();
        let mut files = vec![];
        for result in ignore::Walk::new("./") {
            let dir = result?;

            let Some(file_type) = dir.file_type() else {
                continue
            };

            if file_type.is_file() {
                let path = dir.path();
                let mut file = FileMatches::new(path);
                grep::searcher::SearcherBuilder::new()
                    .after_context(2)
                    .build()
                    .search_path(
                        &matcher,
                        path,
                        Sink(|byte_offset, line| {
                            file.push(matcher.parse_line(byte_offset as usize, &line));
                        }),
                    )?;
                files.push(file);
            }
        }
        Ok(Self { files })
    }

    // pub fn group_by_label(&self) -> HashMap<&str, Vec<ValidMatch>> {
    //     let mut groups = HashMap::new();
    //     for comment in self.valid() {
    //         groups
    //             .entry(&*comment.opts.label)
    //             .or_insert(vec![])
    //             .push(comment)
    //     }
    //     groups
    // }

    // pub fn valid(&self) -> impl Iterator<Item = ValidMatch> + '_ {
    //     self.matches.iter().filter_map(Match::to_valid)
    // }
}

#[derive(Debug)]
struct Match {
    opts: Result<Opts, ParseError>,
    span: Range<usize>,
}

impl Match {
    fn to_valid<'a>(&'a self) -> Option<ValidMatch<'a>> {
        if let Ok(opts) = &self.opts {
            Some(ValidMatch { opts })
        } else {
            None
        }
    }

    fn to_invalid(&self) -> Option<InvalidMatch> {
        if let Err(error) = self.opts {
            Some(InvalidMatch { error, m: self })
        } else {
            None
        }
    }

    fn span(&self) -> Range<usize> {
        self.span.clone()
    }
}

pub struct InvalidMatch<'a> {
    pub error: ParseError,
    m: &'a Match,
}

impl InvalidMatch<'_> {
    pub fn span(&self) -> Range<usize> {
        self.m.span()
    }
}

#[derive(Copy, Clone)]
pub struct ValidMatch<'a> {
    opts: &'a Opts,
}

#[derive(Debug)]
pub struct Opts {
    pub label: String,
    pub count: Option<u64>,
    len: usize,
}

#[derive(Debug, Copy, Clone)]
pub enum ParseError {
    Malformed,
    InvalidCount { start: usize, end: usize },
}

struct Matcher {
    re: regex::Regex,
}

impl Matcher {
    fn new() -> Matcher {
        const REGEX: &str = r"\(\s*([A-Za-z0-9\-_]*)\s*(?:,\s*([^\)]*)\s*)?\)";
        Matcher {
            re: regex::Regex::new(REGEX).unwrap(),
        }
    }

    fn parse_line(&self, byte_offset: usize, line: &str) -> Match {
        let idx = find_codesync(line.as_bytes()).expect("line should contain a match");
        let opts = self.parse_opts(
            byte_offset + idx + CODESYNC.len(),
            &line[idx + CODESYNC.len()..],
        );

        let start = byte_offset as usize + idx;
        let end = start + CODESYNC.len() + if let Ok(opts) = &opts { opts.len } else { 0 };
        Match {
            opts,
            span: start..end,
        }
    }

    fn parse_opts(&self, byte_offset: usize, haystack: &str) -> Result<Opts, ParseError> {
        let Some(captures) = self.re.captures(haystack) else {
            return Err(ParseError::Malformed);
        };
        let label = captures[1].to_string();
        let count = if let Some(m) = captures.get(2) {
            Some(
                m.as_str()
                    .parse::<u64>()
                    .map_err(|_| ParseError::InvalidCount {
                        start: byte_offset + m.start(),
                        end: byte_offset + m.end(),
                    })?,
            )
        } else {
            None
        };
        Ok(Opts {
            label,
            count,
            len: captures.len(),
        })
    }
}

struct Sink<F>(pub F)
where
    F: FnMut(u64, String);

impl<F> grep::searcher::Sink for Sink<F>
where
    F: FnMut(u64, String),
{
    type Error = io::Error;

    fn matched(
        &mut self,
        _searcher: &grep::searcher::Searcher,
        mat: &grep::searcher::SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        let matched = match str::from_utf8(mat.bytes()) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(mat.bytes()).into_owned(),
        };
        (self.0)(mat.absolute_byte_offset(), matched);
        Ok(true)
    }
}

impl grep::matcher::Matcher for &Matcher {
    type Captures = grep::matcher::NoCaptures;

    type Error = grep::matcher::NoError;

    fn find_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<grep::matcher::Match>, Self::Error> {
        Ok(find_codesync(&haystack[at..])
            .map(|idx| grep::matcher::Match::new(at + idx, at + idx + CODESYNC.len())))
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(grep::matcher::NoCaptures::new())
    }
}

fn find_codesync(haystack: &[u8]) -> Option<usize> {
    kmp::search(&haystack, CODESYNC, &CODESYNC_KMP_TABLE)
}
