use const_serialize::{ConstStr, ConstVec, SerializeConst, serialize_const};
use linkme::distributed_slice;

#[derive(Debug, Clone, PartialEq, SerializeConst)]
pub struct Asset {
    path: ConstStr,
}

impl Asset {
    pub const fn new(path: &str) -> Self {
        Self {
            path: ConstStr::new(path),
        }
    }
}

#[macro_export]
macro_rules! asset {
    () => {};
}

#[distributed_slice]
pub static ASSETS: [[u8; 1024]];

#[distributed_slice(ASSETS)]
static ENTRY: [u8; 1024] = const {
    let mut buffer = ConstVec::new();
    buffer = serialize_const(&Asset::new("./kek.png"), buffer);

    let mut out = [0u8; 1024];
    let src = buffer.as_ref();
    let mut i = 0;
    while i < buffer.len() {
        out[i] = src[i];
        i += 1;
    }
    out
};
