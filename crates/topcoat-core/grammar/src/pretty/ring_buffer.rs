use std::{
    collections::VecDeque,
    ops::{Index, IndexMut},
};

/// A lightweight abstraction over [`VecDeque`] that preserves stable indexing after elements
/// are removed from the front.
///
/// Unlike a plain [`VecDeque`], where removing elements from the front causes all remaining
/// elements to shift their indices, `RingBuffer` maintains stable absolute indices by tracking
/// an internal offset. This allows you to refer to elements by their original insertion position
/// even after earlier elements have been removed.
///
/// # Example
///
/// ```
/// # use topcoat_core_grammar::pretty::RingBuffer;
/// let mut buffer = RingBuffer::new();
/// buffer.push_back("first"); // index 0
/// buffer.push_back("second"); // index 1
/// buffer.push_back("third"); // index 2
///
/// buffer.pop_front(); // removes "first"
///
/// // Index 1 still refers to "second" (not shifted to 0)
/// assert_eq!(buffer[1], "second");
/// assert_eq!(buffer[2], "third");
/// ```
pub struct RingBuffer<T> {
    inner: VecDeque<T>,
    offset: usize,
}

impl<T> RingBuffer<T> {
    /// Creates a new empty `RingBuffer`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            offset: 0,
        }
    }

    /// Appends an element to the back of the buffer.
    ///
    /// The element can be accessed using an index equal to the current offset plus length.
    pub fn push_back(&mut self, value: T) {
        self.inner.push_back(value);
    }

    /// Removes and returns the element at the front of the buffer.
    ///
    /// This operation increments the internal offset, preserving the absolute indices
    /// of all remaining elements.
    ///
    /// # Returns
    ///
    /// - `Some(value)` if the buffer is not empty
    /// - `None` if the buffer is empty
    pub fn pop_front(&mut self) -> Option<T> {
        self.offset += 1;
        self.inner.pop_front()
    }

    /// Returns a reference to the last element in the buffer, or `None` if empty.
    #[must_use]
    pub fn last(&self) -> Option<&T> {
        self.inner.iter().last()
    }

    /// Returns the number of elements currently in the buffer.
    ///
    /// Note that this returns the count of elements, not the maximum index value.
    /// After popping elements, the valid index range will be `[offset..offset+len)`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the buffer contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the index of the next element that is inserted into the buffer.
    #[must_use]
    pub fn next_index(&self) -> usize {
        self.len() + self.offset
    }
}

impl<T> Default for RingBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Provides immutable indexing using absolute indices.
///
/// The index parameter should be the absolute position (original insertion index),
/// not relative to the current buffer state. The implementation automatically adjusts
/// for elements that have been popped from the front.
///
/// # Panics
///
/// Panics if the index is out of bounds (either before the current offset or beyond
/// the end of the buffer).
impl<T> Index<usize> for RingBuffer<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index - self.offset]
    }
}

/// Provides mutable indexing using absolute indices.
///
/// The index parameter should be the absolute position (original insertion index),
/// not relative to the current buffer state. The implementation automatically adjusts
/// for elements that have been popped from the front.
///
/// # Panics
///
/// Panics if the index is out of bounds (either before the current offset or beyond
/// the end of the buffer).
impl<T> IndexMut<usize> for RingBuffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.inner[index - self.offset]
    }
}
