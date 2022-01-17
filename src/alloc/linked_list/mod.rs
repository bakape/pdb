mod cursor;
mod node;

mod tests;

use node::Node;
use std::{
    iter::{FromIterator, FusedIterator},
    marker::PhantomData,
    ptr::null_mut,
};

pub use node::{NullRef, Ref};

use self::cursor::CursorMut;

// TODO: write benchmarks to find the right capacity for each application.
// Bigger lists have more cache-local values but also require more Ref
// updates on shifting, which produce cache misses.

// TODO: implement sorting

/// Doubly-linked unrolled list with cursor iteration and stable item
/// referencing.
///
/// [LinkedList] can be used as is or as ordered storage for other collections.
pub struct LinkedList<T>
where
    T: Sized,
{
    /// First node of list
    head: *mut Node<T>,

    /// Last node of the list
    tail: *mut Node<T>,

    /// Cached for cheap lookup
    length: usize,
}

impl<T> Drop for LinkedList<T>
where
    T: Sized,
{
    fn drop(&mut self) {
        if self.head != null_mut() {
            unsafe { Box::from_raw(self.head) }.drop_list();
        }
    }
}

impl<T> LinkedList<T>
where
    T: Sized + 'static,
{
    /// Create new empty list
    #[inline]
    pub fn new() -> Self {
        Self {
            head: null_mut(),
            tail: null_mut(),
            length: 0,
        }
    }

    /// Creates a cursor for iterating and manipulating the list
    #[inline]
    pub fn cursor_mut(&mut self) -> CursorMut<'_, T> {
        unsafe { CursorMut::new(self, self.head) }
    }

    /// Returns the length of the list
    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    // TODO: Immutable iteration

    /// Return a forward mutable iterator over the list
    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = &'_ mut T> + FusedIterator {
        IterMut::<'_, T, Forward>::new(self.cursor_mut())
    }

    /// Return a backward mutable iterator over the list
    pub fn iter_mut_reverse(
        &mut self,
    ) -> impl ExactSizeIterator<Item = &'_ mut T> + FusedIterator {
        IterMut::<'_, T, Backward>::new({
            let mut c = self.cursor_mut();
            c.seek_to_end();
            c
        })
    }
}

/// Advances a cursor in a direction
trait Advance {
    /// Try to advance the cursor in a direction and return, if it was
    fn try_advance<'a, T>(c: &mut CursorMut<'a, T>) -> bool
    where
        T: Sized + 'static;
}

/// Advances the cursor forward
struct Forward;

impl Advance for Forward {
    #[inline]
    fn try_advance<'a, T>(c: &mut CursorMut<'a, T>) -> bool
    where
        T: Sized + 'static,
    {
        c.next()
    }
}

/// Advance the cursor backward
struct Backward;

impl Advance for Backward {
    #[inline]
    fn try_advance<'a, T>(c: &mut CursorMut<'a, T>) -> bool
    where
        T: Sized + 'static,
    {
        c.previous()
    }
}

/// Directional iterator for [LinkedList]
struct IterMut<'a, T, A>
where
    T: Sized + 'static,
    A: Advance,
{
    visited_first: bool,
    cursor: CursorMut<'a, T>,
    pd: PhantomData<A>,
}

impl<'a, T, A> IterMut<'a, T, A>
where
    T: Sized + 'static,
    A: Advance,
{
    fn new(c: CursorMut<'a, T>) -> Self {
        Self {
            visited_first: false,
            cursor: c,
            pd: PhantomData,
        }
    }
}

impl<'a, T, A> Iterator for IterMut<'a, T, A>
where
    T: Sized + 'static,
    A: Advance,
{
    type Item = &'a mut T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if !self.visited_first {
            self.visited_first = true;
        } else {
            if !A::try_advance(&mut self.cursor) {
                return None;
            }
        }

        self.cursor.value()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, T, A> ExactSizeIterator for IterMut<'a, T, A>
where
    T: Sized + 'static,
    A: Advance,
{
    #[inline]
    fn len(&self) -> usize {
        self.cursor.list().len()
    }
}

impl<'a, T, A> FusedIterator for IterMut<'a, T, A>
where
    T: Sized + 'static,
    A: Advance,
{
}

impl<T> FromIterator<T> for LinkedList<T>
where
    T: Sized + 'static,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut ll = LinkedList::new();
        let mut c = ll.cursor_mut();
        for val in iter.into_iter() {
            c.insert_after(val);
            c.next();
        }
        ll
    }
}
