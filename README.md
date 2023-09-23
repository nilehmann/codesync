# Codesync

`codesync` is a tool to help you keep different parts of your codebase in sync by sanity checking
comments spread around the codebase.

In a nutshell, with `codesync` you write comments of the form `CODESYNC(my-label, count)` and
then check that all comments with label `my-label` have the same `count` and if they do
that there are exactly `count` of them.
The `count` is optional and defaults to `2`.

## Concepts

* _Match_: an occurrence of the string `CODESYNC` in a file. The pattern will normally be included
inside a comment in your programming language. To keep it simple `codesync` doesn't understand comments, it just looks for occurrences of the string `CODESYNC` in all the files in your project.
* _Comment_: a (valid) comment is match with appropriate arguments (label and optional count).
* _Invalid Match_: a match that doesn't have appropriate arguments.

## Installation

```bash
cargo install --git https://github.com/nilehmann/codesync
```

## Basic Usage

```console
$ codesync
Usage: codesync <COMMAND>

Commands:
  check  Check that all matches are valid comments and that their counts are correct.
  show   Show all valid codesync comments with a given label. This ignores invalid matches.
  list   List all valid labels. This ignores invalid matches.

Options:
  -h, --help  Print help
```
