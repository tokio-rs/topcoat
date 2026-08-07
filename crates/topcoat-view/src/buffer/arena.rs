use std::{
    fmt,
    ops::{Index, IndexMut},
};

/// The length of the first chunk, as a power of two.
const FIRST_CHUNK_SHIFT: u32 = 4;

/// The length of the first chunk.
const FIRST_CHUNK_LEN: usize = 1 << FIRST_CHUNK_SHIFT;

/// An append-only sequence that never moves the elements it holds.
///
/// The arena stores its elements in chunks, each twice as long as the one
/// before it. Pushing past the end of the last chunk allocates a new one
/// rather than reallocating and copying, so an element keeps its address for
/// as long as the arena lives and pushing is worst-case constant time.
///
/// Random access stays a handful of instructions: chunk lengths are powers
/// of two, so an index splits into a chunk and an offset within it with a
/// base-two logarithm and a mask.
///
/// The chunk vector itself does reallocate, but each chunk holds as many
/// elements as all the chunks before it together, so `n` elements need only
/// `log2(n)` chunks.
pub struct Arena<T> {
    chunks: Vec<Vec<T>>,
    len: usize,
}

impl<T> Arena<T> {
    /// Creates an empty arena, without allocating.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            chunks: Vec::new(),
            len: 0,
        }
    }

    /// Returns the number of elements in the arena.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the arena holds no elements.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the length of the chunk at `chunk`.
    #[inline]
    const fn chunk_len(chunk: usize) -> usize {
        FIRST_CHUNK_LEN << chunk
    }

    /// Splits a global index into the chunk holding it and its offset there.
    ///
    /// Chunk `c` holds the indices `FIRST_CHUNK_LEN * (2^c - 1)` up to
    /// `FIRST_CHUNK_LEN * (2^(c+1) - 1)`, so offsetting the index by
    /// `FIRST_CHUNK_LEN` makes the position of its highest set bit name the
    /// chunk and the bits below it the offset.
    #[inline]
    const fn split(index: usize) -> (usize, usize) {
        let key = index + FIRST_CHUNK_LEN;
        let chunk = (key.ilog2() - FIRST_CHUNK_SHIFT) as usize;
        (chunk, key & (Self::chunk_len(chunk) - 1))
    }

    /// Appends an element, allocating a chunk when the last one is full.
    #[inline]
    pub fn push(&mut self, value: T) {
        let (chunk, offset) = Self::split(self.len);
        if offset == 0 {
            self.chunks.push(Vec::with_capacity(Self::chunk_len(chunk)));
        }
        self.chunks[chunk].push(value);
        self.len += 1;
    }

    /// Returns the element at `index`, or `None` if it is out of bounds.
    #[must_use]
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        let (chunk, offset) = Self::split(index);
        Some(&self.chunks[chunk][offset])
    }

    /// Returns the element at `index` mutably, or `None` if it is out of
    /// bounds.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        let (chunk, offset) = Self::split(index);
        Some(&mut self.chunks[chunk][offset])
    }

    /// Returns an iterator over the elements, in the order they were pushed.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.chunks.iter().flatten()
    }
}

impl<T> Default for Arena<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Index<usize> for Arena<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &T {
        let len = self.len;
        self.get(index)
            .unwrap_or_else(|| panic!("index {index} is out of bounds of an arena of length {len}"))
    }
}

impl<T> IndexMut<usize> for Arena<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut T {
        let len = self.len;
        self.get_mut(index)
            .unwrap_or_else(|| panic!("index {index} is out of bounds of an arena of length {len}"))
    }
}

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = &'a T;
    type IntoIter = std::iter::Flatten<std::slice::Iter<'a, Vec<T>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.chunks.iter().flatten()
    }
}

impl<T: fmt::Debug> fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fills an arena with `0..len`.
    fn filled(len: usize) -> Arena<usize> {
        let mut arena = Arena::new();
        for value in 0..len {
            arena.push(value);
        }
        arena
    }

    #[test]
    fn a_new_arena_is_empty_and_unallocated() {
        let arena = Arena::<usize>::new();
        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());
        assert!(arena.chunks.is_empty());
        assert_eq!(arena.get(0), None);
    }

    #[test]
    fn chunks_double_in_length() {
        // Three full chunks of 1x, 2x and 4x the first chunk's length.
        let arena = filled(FIRST_CHUNK_LEN * 7);
        let lengths: Vec<_> = arena.chunks.iter().map(Vec::len).collect();
        assert_eq!(
            lengths,
            [FIRST_CHUNK_LEN, FIRST_CHUNK_LEN * 2, FIRST_CHUNK_LEN * 4],
        );
    }

    #[test]
    fn a_chunk_is_allocated_only_once_the_previous_one_is_full() {
        let mut arena = filled(FIRST_CHUNK_LEN);
        assert_eq!(arena.chunks.len(), 1);
        arena.push(0);
        assert_eq!(arena.chunks.len(), 2);
    }

    #[test]
    fn splitting_walks_the_chunks_in_order() {
        // The first index of a chunk, its last, and one in between.
        assert_eq!(Arena::<u8>::split(0), (0, 0));
        assert_eq!(Arena::<u8>::split(1), (0, 1));
        assert_eq!(
            Arena::<u8>::split(FIRST_CHUNK_LEN - 1),
            (0, FIRST_CHUNK_LEN - 1)
        );
        assert_eq!(Arena::<u8>::split(FIRST_CHUNK_LEN), (1, 0));
        assert_eq!(
            Arena::<u8>::split(FIRST_CHUNK_LEN * 3 - 1),
            (1, FIRST_CHUNK_LEN * 2 - 1)
        );
        assert_eq!(Arena::<u8>::split(FIRST_CHUNK_LEN * 3), (2, 0));
    }

    #[test]
    fn indexing_finds_every_element_across_chunks() {
        let len = FIRST_CHUNK_LEN * 10;
        let arena = filled(len);
        assert_eq!(arena.len(), len);
        for index in 0..len {
            assert_eq!(arena[index], index);
            assert_eq!(arena.get(index), Some(&index));
        }
        assert_eq!(arena.get(len), None);
    }

    #[test]
    fn elements_are_mutable_in_place() {
        let mut arena = filled(FIRST_CHUNK_LEN * 4);
        arena[FIRST_CHUNK_LEN * 3] = 99;
        assert_eq!(arena[FIRST_CHUNK_LEN * 3], 99);
        assert_eq!(arena.get_mut(FIRST_CHUNK_LEN * 4), None);
    }

    #[test]
    fn iteration_follows_push_order() {
        let len = FIRST_CHUNK_LEN * 5;
        let arena = filled(len);
        assert!(arena.iter().copied().eq(0..len));
        assert!((&arena).into_iter().copied().eq(0..len));
    }

    #[test]
    fn pushing_never_moves_an_element() {
        let mut arena = filled(1);
        let first: *const usize = &raw const arena[0];
        for value in 1..FIRST_CHUNK_LEN * 10 {
            arena.push(value);
            assert!(std::ptr::eq(&raw const arena[0], first));
        }
    }

    #[test]
    #[should_panic(expected = "index 3 is out of bounds of an arena of length 3")]
    fn indexing_past_the_end_panics() {
        let arena = filled(3);
        let _ = arena[3];
    }

    #[test]
    fn debug_prints_the_elements_as_a_list() {
        assert_eq!(format!("{:?}", filled(3)), "[0, 1, 2]");
    }
}
