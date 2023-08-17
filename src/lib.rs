use std::{
    collections::HashMap,
    io,
    ops::Range,
    path::{Path, PathBuf},
    str,
};

mod kmp;

const CODESYNC: [u8; 8] = [b'C', b'O', b'D', b'E', b'S', b'Y', b'N', b'C'];
const CODESYNC_KMP_TABLE: [usize; CODESYNC.len()] = kmp::table(CODESYNC);

pub struct Matches {
    pub files: Vec<FileMatch>,
}

pub struct FileMatch {
    pub path: PathBuf,
    matches: Vec<Match>,
}

impl FileMatch {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            matches: vec![],
        }
    }

    fn push(&mut self, m: Match) {
        self.matches.push(m)
    }

    pub fn read_content(&self) -> std::io::Result<String> {
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
                let mut file = FileMatch::new(path);
                grep::searcher::Searcher::new().search_path(
                    &matcher,
                    path,
                    Sink(|byte_offset, line| {
                        file.push(matcher.parse_line(byte_offset as usize, &line));
                    }),
                )?;
                if !file.matches.is_empty() {
                    files.push(file);
                }
            }
        }
        Ok(Self { files })
    }

    pub fn group_by_label(&self) -> HashMap<&str, Vec<Comment>> {
        let mut groups = HashMap::new();
        for comment in self.valid() {
            groups
                .entry(&*comment.opts.label)
                .or_insert(vec![])
                .push(comment)
        }
        groups
    }

    /// Iterator over all valid comments
    pub fn valid(&self) -> impl Iterator<Item = Comment> + '_ {
        self.files
            .iter()
            .flat_map(|file| file.matches.iter().filter_map(|m| m.to_valid(&file.path)))
    }

    /// Iterator over all invalid comments
    pub fn invalid(&self) -> impl Iterator<Item = InvalidComment> + '_ {
        self.files
            .iter()
            .flat_map(|file| file.matches.iter().filter_map(|m| m.to_invalid(&file.path)))
    }
}

#[derive(Debug)]
struct Match {
    opts: Result<Opts, ParseError>,
    /// The offset in bytes from the beginning of the file to the start of the match
    byte_offset: usize,
}

impl Match {
    fn to_valid<'a>(&'a self, file: &'a Path) -> Option<Comment<'a>> {
        if let Ok(opts) = &self.opts {
            Some(Comment {
                opts,
                file,
                m: self,
            })
        } else {
            None
        }
    }

    fn to_invalid<'a>(&'a self, file: &'a Path) -> Option<InvalidComment> {
        if let Err(error) = self.opts {
            Some(InvalidComment {
                error,
                m: self,
                file,
            })
        } else {
            None
        }
    }

    fn span(&self) -> Range<usize> {
        let start = self.byte_offset;
        let mut end = start + CODESYNC.len();
        if let Ok(opts) = &self.opts {
            end += opts.len;
        }
        start..end
    }
}

/// A valid codesync comment
#[derive(Copy, Clone)]
pub struct Comment<'a> {
    pub opts: &'a Opts,
    file: &'a Path,
    m: &'a Match,
}

impl Comment<'_> {
    pub fn span(&self) -> Range<usize> {
        self.m.span()
    }

    pub fn file(&self) -> &Path {
        self.file
    }
}

/// An invalid codesync comment
pub struct InvalidComment<'a> {
    pub error: ParseError,
    file: &'a Path,
    m: &'a Match,
}

impl InvalidComment<'_> {
    pub fn span(&self) -> Range<usize> {
        self.m.span()
    }

    pub fn file(&self) -> &Path {
        self.file
    }
}

#[derive(Debug)]
pub struct Opts {
    pub label: String,
    count: Option<u32>,
    /// The length of the parsed string including delimiting parentheses
    len: usize,
}

impl Opts {
    pub fn count(&self) -> u32 {
        self.count.unwrap_or(2)
    }
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
        const OPTS_REGEX: &str = r"^\(\s*([A-Za-z0-9\-_]*)\s*(?:,\s*([^\)]*)\s*)?\)";
        Matcher {
            re: regex::Regex::new(OPTS_REGEX).unwrap(),
        }
    }

    fn parse_line(&self, byte_offset: usize, line: &str) -> Match {
        let idx = find_codesync(line.as_bytes()).expect("line should contain a match");
        let opts = self.parse_opts(
            byte_offset + idx + CODESYNC.len(),
            &line[idx + CODESYNC.len()..],
        );

        Match {
            opts,
            byte_offset: byte_offset + idx,
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
                    .parse::<u32>()
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
            len: captures[0].len(),
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
    kmp::search(&haystack, &CODESYNC, &CODESYNC_KMP_TABLE)
}
