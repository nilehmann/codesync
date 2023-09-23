use std::collections::HashSet;

/// Provides conversion to and detection of camel case strings.
///
/// Example string `camelCase`
pub mod camel;
pub use camel::is_camel_case;
pub use camel::to_camel_case;

/// Provides conversion to and detection of snake case strings.
///
/// Example string `snake_case`
pub mod snake;

use regex::Regex;
pub use snake::is_snake_case;
pub use snake::to_snake_case;

/// Provides conversion to and detection of screaming snake case strings.
///
/// Example string `SCREAMING_SNAKE_CASE`
pub mod screaming_snake;
pub use screaming_snake::is_screaming_snake_case;
pub use screaming_snake::to_screaming_snake_case;

/// Provides conversion to and detection of kebab case strings.
///
/// Example string `kebab-case`
pub mod kebab;
pub use kebab::is_kebab_case;
pub use kebab::to_kebab_case;

/// Provides conversion to and detection of train case strings.
///
/// Example string `Train-Case`
pub mod train;
pub use train::is_train_case;
pub use train::to_train_case;

/// Provides conversion to and detection of sentence case strings.
///
/// Example string `Sentence case`
pub mod sentence;
pub use sentence::is_sentence_case;
pub use sentence::to_sentence_case;

/// Provides conversion to pascal case strings.
///
/// Example string `PascalCase`
pub mod pascal;
pub use pascal::is_pascal_case;
pub use pascal::to_pascal_case;

#[doc(hidden)]
pub struct CamelOptions {
    pub new_word: bool,
    pub last_char: char,
    pub first_word: bool,
    pub injectable_char: char,
    pub has_separator: bool,
    pub inverted: bool,
}

#[doc(hidden)]
pub fn to_case_snake_like(convertable_string: &str, replace_with: &str, case: &str) -> String {
    let mut first_character: bool = true;
    let mut result: String = String::with_capacity(convertable_string.len() * 2);
    for char_with_index in trim_right(convertable_string).char_indices() {
        if char_is_separator(&char_with_index.1) {
            if !first_character {
                first_character = true;
                result.push(replace_with.chars().next().unwrap_or('_'));
            }
        } else if requires_separator(char_with_index, first_character, convertable_string) {
            first_character = false;
            result = snake_like_with_separator(result, replace_with, &char_with_index.1, case)
        } else {
            first_character = false;
            result = snake_like_no_separator(result, &char_with_index.1, case)
        }
    }
    result
}

#[doc(hidden)]
pub fn to_case_camel_like(
    convertable_string: &str,
    camel_options: CamelOptions,
    acronyms: &HashSet<String>,
) -> String {
    let mut new_word: bool = camel_options.new_word;
    let mut first_word: bool = camel_options.first_word;
    let mut last_char: char = camel_options.last_char;
    let mut found_real_char: bool = false;
    let mut result: String = String::with_capacity(convertable_string.len() * 2);
    for character in trim_right(convertable_string).chars() {
        if char_is_separator(&character) && found_real_char {
            new_word = true;
        } else if !found_real_char && is_not_alphanumeric(character) {
            continue;
        } else if last_char_lower_current_is_upper_or_new_word(new_word, last_char, character) {
            found_real_char = true;
            new_word = false;

            result = append_on_new_word(result, first_word, character, &camel_options);
            first_word = false;
        } else {
            found_real_char = true;
            last_char = character;
            result.push(character.to_ascii_lowercase());
        }
    }

    result = capitalize_acronym_substrings(&result, acronyms);

    result
}

fn capitalize_acronym_substrings(str: &str, acronyms: &HashSet<String>) -> String {
    let mut new_string = str.to_string();
    for acronym in acronyms {
        // There will probably be a bug here. See test_inflected_acronyms_buggy_behavior
        // let pattern = format!(r"(?:\b)(?i)({})(?-i)(?:\b)", acronym);
        let pattern = format!(r"(?i)({})(?-i)", acronym);

        let re = Regex::new(&pattern).unwrap();
        let mut result = re.replace_all(&new_string, |caps: &regex::Captures| {
            let matched = caps.get(1).unwrap().as_str();
            matched.to_uppercase()
        });

        new_string = result.to_mut().to_string();
    }

    new_string
}

#[inline]
fn append_on_new_word(
    mut result: String,
    first_word: bool,
    character: char,
    camel_options: &CamelOptions,
) -> String {
    if not_first_word_and_has_separator(first_word, camel_options.has_separator) {
        result.push(camel_options.injectable_char);
    }
    if first_word_or_not_inverted(first_word, camel_options.inverted) {
        result.push(character.to_ascii_uppercase());
    } else {
        result.push(character.to_ascii_lowercase());
    }
    result
}

fn not_first_word_and_has_separator(first_word: bool, has_separator: bool) -> bool {
    has_separator && !first_word
}

fn first_word_or_not_inverted(first_word: bool, inverted: bool) -> bool {
    !inverted || first_word
}

fn last_char_lower_current_is_upper_or_new_word(
    new_word: bool,
    last_char: char,
    character: char,
) -> bool {
    new_word || ((last_char.is_lowercase() && character.is_uppercase()) && (last_char != ' '))
}

fn char_is_separator(character: &char) -> bool {
    is_not_alphanumeric(*character)
}

fn trim_right(convertable_string: &str) -> &str {
    convertable_string.trim_end_matches(is_not_alphanumeric)
}

fn is_not_alphanumeric(character: char) -> bool {
    !character.is_alphanumeric()
}

#[inline]
fn requires_separator(
    char_with_index: (usize, char),
    first_character: bool,
    convertable_string: &str,
) -> bool {
    !first_character
        && char_with_index.1.is_uppercase()
        && next_or_previous_char_is_lowercase(convertable_string, char_with_index.0)
}

#[inline]
fn snake_like_no_separator(mut accumlator: String, current_char: &char, case: &str) -> String {
    if case == "lower" {
        accumlator.push(current_char.to_ascii_lowercase());
        accumlator
    } else {
        accumlator.push(current_char.to_ascii_uppercase());
        accumlator
    }
}

#[inline]
fn snake_like_with_separator(
    mut accumlator: String,
    replace_with: &str,
    current_char: &char,
    case: &str,
) -> String {
    if case == "lower" {
        accumlator.push(replace_with.chars().next().unwrap_or('_'));
        accumlator.push(current_char.to_ascii_lowercase());
        accumlator
    } else {
        accumlator.push(replace_with.chars().next().unwrap_or('_'));
        accumlator.push(current_char.to_ascii_uppercase());
        accumlator
    }
}

fn next_or_previous_char_is_lowercase(convertable_string: &str, char_with_index: usize) -> bool {
    convertable_string
        .chars()
        .nth(char_with_index + 1)
        .unwrap_or('A')
        .is_lowercase()
        || convertable_string
            .chars()
            .nth(char_with_index - 1)
            .unwrap_or('A')
            .is_lowercase()
}

// fn char_is_uppercase(test_char: char) -> bool {
//     test_char.is_uppercase()
// }

#[test]
fn test_trim_bad_chars() {
    assert_eq!("abc", trim_right("abc----^"))
}

#[test]
fn test_trim_bad_chars_when_none_are_bad() {
    assert_eq!("abc", trim_right("abc"))
}

#[test]
fn test_is_not_alphanumeric_on_is_alphanumeric() {
    assert!(!is_not_alphanumeric('a'))
}

#[test]
fn test_is_not_alphanumeric_on_is_not_alphanumeric() {
    assert!(is_not_alphanumeric('_'))
}

#[test]
fn test_next_or_previous_char_is_lowercase_true() {
    assert!(next_or_previous_char_is_lowercase("TestWWW", 3))
}

#[test]
fn test_next_or_previous_char_is_lowercase_false() {
    assert!(!next_or_previous_char_is_lowercase("TestWWW", 5))
}

#[test]
fn snake_like_with_separator_lowers() {
    assert_eq!(
        snake_like_with_separator("".to_owned(), "^", &'c', "lower"),
        "^c".to_string()
    )
}

#[test]
fn snake_like_with_separator_upper() {
    assert_eq!(
        snake_like_with_separator("".to_owned(), "^", &'c', "upper"),
        "^C".to_string()
    )
}

#[test]
fn snake_like_no_separator_lower() {
    assert_eq!(
        snake_like_no_separator("".to_owned(), &'C', "lower"),
        "c".to_string()
    )
}

#[test]
fn snake_like_no_separator_upper() {
    assert_eq!(
        snake_like_no_separator("".to_owned(), &'c', "upper"),
        "C".to_string()
    )
}

#[test]
fn requires_separator_upper_not_first_wrap_is_safe_current_upper() {
    assert!(requires_separator((2, 'C'), false, "test"))
}

#[test]
fn requires_separator_upper_not_first_wrap_is_safe_current_lower() {
    assert!(!requires_separator((2, 'c'), false, "test"))
}

#[test]
fn requires_separator_upper_first_wrap_is_safe_current_upper() {
    assert!(!requires_separator((0, 'T'), true, "Test"))
}

#[test]
fn requires_separator_upper_first_wrap_is_safe_current_lower() {
    assert!(!requires_separator((0, 't'), true, "Test"))
}

#[test]
fn requires_separator_upper_first_wrap_is_safe_current_lower_next_is_too() {
    assert!(!requires_separator((0, 't'), true, "test"))
}

#[test]
fn test_char_is_separator_dash() {
    assert!(char_is_separator(&'-'))
}

#[test]
fn test_char_is_separator_underscore() {
    assert!(char_is_separator(&'_'))
}

#[test]
fn test_char_is_separator_space() {
    assert!(char_is_separator(&' '))
}

#[test]
fn test_char_is_separator_when_not() {
    assert!(!char_is_separator(&'A'))
}

#[test]
fn test_last_char_lower_current_is_upper_or_new_word_with_new_word() {
    assert!(last_char_lower_current_is_upper_or_new_word(true, ' ', '-'))
}

#[test]
fn test_last_char_lower_current_is_upper_or_new_word_last_char_space() {
    assert!(!last_char_lower_current_is_upper_or_new_word(
        false, ' ', '-'
    ))
}

#[test]
fn test_last_char_lower_current_is_upper_or_new_word_last_char_lower_current_upper() {
    assert!(last_char_lower_current_is_upper_or_new_word(
        false, 'a', 'A'
    ))
}

#[test]
fn test_last_char_lower_current_is_upper_or_new_word_last_char_upper_current_upper() {
    assert!(!last_char_lower_current_is_upper_or_new_word(
        false, 'A', 'A'
    ))
}

#[test]
fn test_last_char_lower_current_is_upper_or_new_word_last_char_upper_current_lower() {
    assert!(!last_char_lower_current_is_upper_or_new_word(
        false, 'A', 'a'
    ))
}

#[test]
fn test_first_word_or_not_inverted_with_first_word() {
    assert!(first_word_or_not_inverted(true, false))
}

#[test]
fn test_first_word_or_not_inverted_not_first_word_not_inverted() {
    assert!(first_word_or_not_inverted(false, false))
}

#[test]
fn test_first_word_or_not_inverted_not_first_word_is_inverted() {
    assert!(!first_word_or_not_inverted(false, true))
}

#[test]
fn test_not_first_word_and_has_separator_is_first_and_not_separator() {
    assert!(!not_first_word_and_has_separator(true, false))
}

#[test]
fn test_not_first_word_and_has_separator_not_first_and_not_separator() {
    assert!(!not_first_word_and_has_separator(false, false))
}

#[test]
fn test_not_first_word_and_has_separator_not_first_and_has_separator() {
    assert!(not_first_word_and_has_separator(false, true))
}
