use topcoat_asset::Asset;

use crate::{FontStyle, FontWeightRange, UnicodeRanges};

pub struct FontFace {
    font_family: &'static str,
    src: Asset,
    weight: Option<FontWeightRange>,
    style: Option<FontStyle>,
    unicode_range: Option<UnicodeRanges>,
}
