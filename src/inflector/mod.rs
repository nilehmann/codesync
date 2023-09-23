//! Adds String based inflections for Rust. Snake, kebab, train, camel,
//! sentence, class, and title cases as well as ordinalize,
//! deordinalize, demodulize, deconstantize, and foreign key are supported as
//! both traits and pure functions acting on String types.
//! ```

/// Provides case inflections
/// - Camel case
/// - Class case
/// - Kebab case
/// - Train case
/// - Screaming snake case
/// - Table case
/// - Sentence case
/// - Snake case
/// - Pascal case
pub mod case;

pub use case::camel::is_camel_case;
pub use case::camel::to_camel_case;

pub use case::pascal::is_pascal_case;
pub use case::pascal::to_pascal_case;

pub use case::snake::is_snake_case;
pub use case::snake::to_snake_case;

pub use case::screaming_snake::is_screaming_snake_case;
pub use case::screaming_snake::to_screaming_snake_case;

pub use case::kebab::is_kebab_case;
pub use case::kebab::to_kebab_case;

pub use case::train::is_train_case;
pub use case::train::to_train_case;

pub use case::sentence::is_sentence_case;
pub use case::sentence::to_sentence_case;

#[allow(missing_docs)]
pub trait Inflector {
    fn to_camel_case(&self) -> String;
    fn is_camel_case(&self) -> bool;

    fn to_pascal_case(&self) -> String;
    fn is_pascal_case(&self) -> bool;

    fn to_snake_case(&self) -> String;
    fn is_snake_case(&self) -> bool;

    fn to_screaming_snake_case(&self) -> String;
    fn is_screaming_snake_case(&self) -> bool;

    fn to_kebab_case(&self) -> String;
    fn is_kebab_case(&self) -> bool;

    fn to_train_case(&self) -> String;
    fn is_train_case(&self) -> bool;

    fn to_sentence_case(&self) -> String;
    fn is_sentence_case(&self) -> bool;

    fn to_title_case(&self) -> String;
    fn is_title_case(&self) -> bool;

    fn ordinalize(&self) -> String;
    fn deordinalize(&self) -> String;

    fn to_foreign_key(&self) -> String;
    fn is_foreign_key(&self) -> bool;

    fn demodulize(&self) -> String;

    fn deconstantize(&self) -> String;

    fn to_class_case(&self) -> String;

    fn is_class_case(&self) -> bool;

    fn to_table_case(&self) -> String;

    fn is_table_case(&self) -> bool;

    fn to_plural(&self) -> String;

    fn to_singular(&self) -> String;
}

#[allow(missing_docs)]
pub trait InflectorNumbers {
    fn ordinalize(&self) -> String;
}
