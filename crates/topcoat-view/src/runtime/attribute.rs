use topcoat_core::context::Cx;

use crate::runtime::{Formatter, Fragment, ViewPart};

#[derive(Debug, Clone)]
pub struct Attribute {
    key: ViewPart,
    value: ViewPart,
}

impl Attribute {
    pub fn new(key: ViewPart, value: ViewPart) -> Self {
        Self { key, value }
    }

    #[inline]
    pub fn key(&self) -> &ViewPart {
        &self.key
    }

    #[inline]
    pub fn value(&self) -> &ViewPart {
        &self.value
    }
}

impl Fragment for Attribute {
    fn fmt(&self, cx: &Cx, f: &mut Formatter<'_>) {
        // HTML attributes like "checked" are only interpreted as "false" when they are completely
        // omitted from the markup. To improve usability we automatically leave out attributes with
        // a "falsy" value.
        if matches!(self.value, ViewPart::Empty | ViewPart::Bool(false)) {
            return;
        }

        // Attributes write their own leading space so that omitting the attribute does not leave
        // an extra unnecessary gap.
        f.write_char_unescaped(' ');
        self.key.fmt(cx, f);
        f.write_str_unescaped("=\"");
        self.value.fmt(cx, f);
        f.write_char_unescaped('"');
    }

    fn size_hint(&self) -> usize {
        [
            1, // <space>
            self.key.size_hint(),
            1, // =
            1, // "
            self.value.size_hint(),
            1, // "
        ]
        .iter()
        .sum()
    }
}

pub trait IntoAttributeKey {
    /// Consumes `self` and produces the corresponding [`ViewPart`].
    fn into_attribute_key(self) -> ViewPart;
}

pub trait IntoAttributeValue {
    /// Consumes `self` and produces the corresponding [`ViewPart`].
    fn into_attribute_value(self) -> ViewPart;
}
