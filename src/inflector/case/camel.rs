use super::*;

/// Converts a `&str` to camelCase `String`
///
/// ```
/// use codesync::inflector::case::to_camel_case;
/// use std::collections::HashSet;
/// assert_eq!(to_camel_case("fooBar", &HashSet::new()), "fooBar");
/// assert_eq!(to_camel_case("FOO_BAR", &HashSet::new()), "fooBar");
/// assert_eq!(to_camel_case("Foo Bar", &HashSet::new()), "fooBar");
/// assert_eq!(to_camel_case("foo_bar", &HashSet::new()), "fooBar");
/// assert_eq!(to_camel_case("Foo bar", &HashSet::new()), "fooBar");
/// assert_eq!(to_camel_case("foo-bar", &HashSet::new()), "fooBar");
/// assert_eq!(to_camel_case("FooBar", &HashSet::new()), "fooBar");
/// assert_eq!(to_camel_case("FooBar3", &HashSet::new()), "fooBar3");
/// assert_eq!(to_camel_case("Foo-Bar", &HashSet::new()), "fooBar");
/// ```
pub fn to_camel_case(non_camelized_string: &str, acronyms: &HashSet<String>) -> String {
    let options = CamelOptions {
        new_word: false,
        last_char: ' ',
        first_word: false,
        injectable_char: ' ',
        has_separator: false,
        inverted: false,
    };
    to_case_camel_like(non_camelized_string, options, acronyms)
}

/// Determines if a `&str` is camelCase bool``
///
/// ```
/// use codesync::inflector::case::is_camel_case;
///
/// assert!(is_camel_case("foo"));
/// assert!(is_camel_case("fooBarIsAReallyReally3longString"));
/// assert!(is_camel_case("fooBarIsAReallyReallyLongString"));
///
/// assert!(!is_camel_case("Foo"));
/// assert!(!is_camel_case("foo-bar-string-that-is-really-really-long"));
/// assert!(!is_camel_case("FooBarIsAReallyReallyLongString"));
/// assert!(!is_camel_case("FOO_BAR_STRING_THAT_IS_REALLY_REALLY_LONG"));
/// assert!(!is_camel_case("foo_bar_string_that_is_really_really_long"));
/// assert!(!is_camel_case("Foo bar string that is really really long"));
/// assert!(!is_camel_case("Foo Bar Is A Really Really Long String"));
/// ```
pub fn is_camel_case(test_string: &str) -> bool {
    to_camel_case(test_string, &HashSet::new()) == test_string
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::inflector::case::CamelOptions;

    use super::is_camel_case;
    use super::to_camel_case;

    #[test]
    fn from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn from_case_with_loads_of_space() {
        let convertable_string: String = "foo           bar".to_owned();
        let expected: String = "fooBar".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn a_name_with_a_dot() {
        let convertable_string: String = "Robert C. Martin".to_owned();
        let expected: String = "robertCMartin".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn random_text_with_bad_chars() {
        let convertable_string: String = "Random text with *(bad) chars".to_owned();
        let expected: String = "randomTextWithBadChars".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn trailing_bad_chars() {
        let convertable_string: String = "trailing bad_chars*(()())".to_owned();
        let expected: String = "trailingBadChars".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn leading_bad_chars() {
        let convertable_string: String = "-!#$%leading bad chars".to_owned();
        let expected: String = "leadingBadChars".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn wrapped_in_bad_chars() {
        let convertable_string: String =
            "-!#$%wrapped in bad chars&*^*&(&*^&(<><?>><?><>))".to_owned();
        let expected: String = "wrappedInBadChars".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn has_a_sign() {
        let convertable_string: String = "has a + sign".to_owned();
        let expected: String = "hasASign".to_owned();
        assert_eq!(
            to_camel_case(&convertable_string, &HashSet::new()),
            expected
        )
    }

    #[test]
    fn has_an_integer_followed_by_an_underscore() {
        let options = CamelOptions {
            new_word: true,
            last_char: ' ',
            first_word: false,
            injectable_char: ' ',
            has_separator: false,
            inverted: false,
        };
        let convertable_string: String = "abc123_xyz".to_owned();
        let expected: String = "Abc123Xyz".to_owned();
        assert_eq!(
            crate::inflector::case::to_case_camel_like(
                &convertable_string,
                options,
                &HashSet::new()
            ),
            expected
        )
    }

    #[test]
    fn is_correct_from_camel_case() {
        let convertable_string: String = "fooBar".to_owned();
        assert!(is_camel_case(&convertable_string))
    }

    #[test]
    fn is_correct_from_pascal_case() {
        let convertable_string: String = "FooBar".to_owned();
        assert!(!is_camel_case(&convertable_string))
    }

    #[test]
    fn is_correct_from_kebab_case() {
        let convertable_string: String = "foo-bar".to_owned();
        assert!(!is_camel_case(&convertable_string))
    }

    #[test]
    fn is_correct_from_sentence_case() {
        let convertable_string: String = "Foo bar".to_owned();
        assert!(!is_camel_case(&convertable_string))
    }

    #[test]
    fn is_correct_from_title_case() {
        let convertable_string: String = "Foo Bar".to_owned();
        assert!(!is_camel_case(&convertable_string))
    }

    #[test]
    fn is_correct_from_train_case() {
        let convertable_string: String = "Foo-Bar".to_owned();
        assert!(!is_camel_case(&convertable_string))
    }

    #[test]
    fn is_correct_from_screaming_snake_case() {
        let convertable_string: String = "FOO_BAR".to_owned();
        assert!(!is_camel_case(&convertable_string))
    }

    #[test]
    fn is_correct_from_snake_case() {
        let convertable_string: String = "foo_bar".to_owned();
        assert!(!is_camel_case(&convertable_string))
    }
}
