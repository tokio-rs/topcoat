use std::{
    collections::VecDeque,
    ops::{Index, IndexMut},
};

/// A queue whose indices stay stable when elements are removed from the
/// front.
///
/// In a plain [`VecDeque`], removing the front element shifts the index of
/// every other element. A `RingBuffer` instead indexes each element by its
/// insertion position, which stays valid until that element is removed.
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

    /// Appends an element to the back of the buffer, at index
    /// [`next_index`](Self::next_index).
    pub fn push_back(&mut self, value: T) {
        self.inner.push_back(value);
    }

    /// Removes and returns the element at the front of the buffer, or `None`
    /// if it is empty.
    ///
    /// The indices of the remaining elements do not change.
    pub fn pop_front(&mut self) -> Option<T> {
        self.offset += 1;
        self.inner.pop_front()
    }

    /// Returns a reference to the last element in the buffer, or `None` if empty.
    #[must_use]
    pub fn last(&self) -> Option<&T> {
        self.inner.iter().last()
    }

    /// Returns the number of elements in the buffer.
    ///
    /// This is not the index of the last element: after elements were
    /// removed from the front, the valid indices start above zero.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the buffer contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the index the next element pushed to the buffer gets.
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

/// Returns the element at its insertion index.
///
/// # Panics
///
/// Panics if the element was already removed or the index is past the end.
impl<T> Index<usize> for RingBuffer<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index - self.offset]
    }
}

/// Returns the element at its insertion index, mutably.
///
/// # Panics
///
/// Panics if the element was already removed or the index is past the end.
impl<T> IndexMut<usize> for RingBuffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.inner[index - self.offset]
    }
}
