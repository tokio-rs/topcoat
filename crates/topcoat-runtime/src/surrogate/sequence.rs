use serde::{Deserialize, de};

use crate::{Surrogate, Surrogated};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sequence<V> {
    t: String,
    bits: u32,
    v: Vec<V>,
}

pub(super) fn deserialize_sequence<'de, T, D>(
    deserializer: D,
    tag: &'static str,
) -> Result<Vec<T>, D::Error>
where
    T: Surrogated,
    T::Surrogate: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    let value = Sequence::<T::Surrogate>::deserialize(deserializer)?;
    if value.t != tag {
        return Err(de::Error::invalid_value(
            de::Unexpected::Str(&value.t),
            &tag,
        ));
    }
    if value.bits != usize::BITS {
        return Err(de::Error::custom(
            "collection pointer width does not match target",
        ));
    }
    Ok(value.v.into_iter().map(Surrogate::into_real).collect())
}
