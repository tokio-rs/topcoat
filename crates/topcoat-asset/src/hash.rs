pub const FNV1A_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

pub const fn fnv1a(bytes: &[u8]) -> u64 {
    fnv1a_continue(FNV1A_OFFSET, bytes)
}

pub const fn fnv1a_continue(mut h: u64, bytes: &[u8]) -> u64 {
    let mut i = 0;
    while i < bytes.len() {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
        i += 1;
    }
    h
}
