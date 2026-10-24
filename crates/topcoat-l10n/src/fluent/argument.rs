use std::{borrow::Cow, fmt::Write, sync::Arc};

use fluent_bundle::{
    FluentValue,
    types::{AnyEq, FluentType},
};

/// A value for a Fluent variable, such as `$name` in `Hello, { $name }!`.
///
/// Strings, primitive numbers and options of them convert into arguments, which is how
/// `t!` accepts them. Borrowed strings stay borrowed. For other types, implement
/// [`FluentType`] and use [`custom`](Self::custom).
#[derive(Clone, Debug)]
pub struct Argument<'a>(Value<'a>);

#[derive(Clone, Debug)]
enum Value<'a> {
    String(Cow<'a, str>),
    Number(f64),
    Custom(Arc<dyn FluentType + Send + Sync>),
    None,
}

impl Argument<'_> {
    /// Creates an argument from a custom Fluent type, such as a date.
    ///
    /// Fluent formats the value with [`FluentType::as_string_threadsafe`].
    #[must_use]
    pub fn custom(value: impl FluentType + Send + Sync) -> Self {
        Self(Value::Custom(Arc::new(value)))
    }

    /// Borrows the argument as a Fluent value.
    pub(crate) fn to_fluent(&self) -> FluentValue<'_> {
        match &self.0 {
            Value::String(text) => FluentValue::String(Cow::Borrowed(text)),
            Value::Number(number) => FluentValue::from(*number),
            Value::Custom(value) => FluentValue::Custom(Box::new(Shared(Arc::clone(value)))),
            Value::None => FluentValue::None,
        }
    }

    /// Appends the argument to a placeholder for a missing message. Strings appear in
    /// quotes and numbers without locale formatting.
    pub(crate) fn write_placeholder(&self, text: &mut String) {
        let _ = match &self.0 {
            Value::String(value) => write!(text, "{value:?}"),
            Value::Number(number) => write!(text, "{number}"),
            Value::Custom(value) => write!(text, "{value:?}"),
            Value::None => text.write_str("None"),
        };
    }
}

impl<'a> From<Cow<'a, str>> for Argument<'a> {
    fn from(text: Cow<'a, str>) -> Self {
        Self(Value::String(text))
    }
}

impl<'a> From<&'a str> for Argument<'a> {
    fn from(text: &'a str) -> Self {
        Self::from(Cow::Borrowed(text))
    }
}

impl<'a> From<&'a String> for Argument<'a> {
    fn from(text: &'a String) -> Self {
        Self::from(text.as_str())
    }
}

impl From<String> for Argument<'_> {
    fn from(text: String) -> Self {
        Self::from(Cow::<str>::Owned(text))
    }
}

impl<'a, T: Into<Argument<'a>>> From<Option<T>> for Argument<'a> {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self(Value::None), Into::into)
    }
}

macro_rules! from_numbers {
    ($($number:ty)*) => {
        $(
            impl From<$number> for Argument<'_> {
                #[allow(
                    clippy::cast_lossless,
                    clippy::cast_precision_loss,
                    clippy::unnecessary_cast,
                    reason = "Fluent formats every number as `f64`"
                )]
                fn from(number: $number) -> Self {
                    Self(Value::Number(number as f64))
                }
            }

            impl From<&$number> for Argument<'_> {
                fn from(number: &$number) -> Self {
                    Self::from(*number)
                }
            }
        )*
    };
}

from_numbers!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64);

/// Hands a shared custom value to Fluent, which takes ownership of each value.
#[derive(Debug)]
struct Shared(Arc<dyn FluentType + Send + Sync>);

impl PartialEq for Shared {
    fn eq(&self, other: &Self) -> bool {
        AnyEq::equals(&*self.0, AnyEq::as_any(&*other.0))
    }
}

impl FluentType for Shared {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(Self(Arc::clone(&self.0)))
    }

    fn as_string(&self, intls: &intl_memoizer::IntlLangMemoizer) -> Cow<'static, str> {
        self.0.as_string(intls)
    }

    fn as_string_threadsafe(
        &self,
        intls: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> Cow<'static, str> {
        self.0.as_string_threadsafe(intls)
    }
}
