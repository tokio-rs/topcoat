use std::borrow::Cow;

use topcoat_asset::Asset;

use crate::UnicodeRange;

pub struct FontFace {
    font_family: Cow<'static, str>,
    src: Asset,
    weight: Option<FontWeight>,
    style: Option<FontStyle>,
    unicode_range: Option<UnicodeRange>,
}
