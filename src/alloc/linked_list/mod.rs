mod cursor;
mod node;

mod tests;

use node::Node;
use std::{
    iter::{FromIterator, FusedIterator},
    marker::PhantomData,
    ptr::null_mut,
};

pub use node::{NodeRef, NullNodeRef};

use self::cursor::CursorMut;

// TODO: write benchmarks to find the right capacity for each application.
// Bigger lists have more cache-local values but also require more NodeRef
// updates on shifting, which produce cache misses.

/// Doubly-linked unrolled list with cursor iteration and reference storage
/// support.
/// Stores type T in N-sized nodes.
pub struct LinkedList<T, const N: usize>
where
    T: Sized,
{
    /// First node of list. Guaranteed to never be null.
    head: *mut Node<T, N>,

    /// Last node of the list. Guaranteed to never be null.
    tail: *mut Node<T, N>,

    /// Cached for cheap lookup
    length: usize,
}

unsafe impl<T, const N: usize> Send for LinkedList<T, N> where T: Sized + Send {}

impl<T, const N: usize> Drop for LinkedList<T, N>
where
    T: Sized,
{
    fn drop(&mut self) {
        if self.head != null_mut() {
            unsafe { Box::from_raw(self.head).drop_list() };
        }
    }
}

impl<T, const N: usize> LinkedList<T, N>
where
    T: Sized + 'static,
{
    /// Create new empty list
    #[inline]
    pub fn new() -> Self {
        // List must never have zero nodes
        let n = Node::empty();
        Self {
            head: n,
            tail: n,
            length: 0,
        }
    }

    /// Creates a cursor for iterating and manipulating the list
    #[inline]
    pub fn cursor_mut(&mut self) -> CursorMut<'_, T, N> {
        unsafe { CursorMut::new(self, self.head, 0) }
    }

    /// Returns the length of the list
    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Return a forward mutable iterator over the list
    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = &'_ mut T> + FusedIterator<Item = &'_ mut T>
    {
        IterMut::<'_, T, Forward, N>::new(self.cursor_mut())
    }

    /// Return a backward mutable iterator over the list
    pub fn iter_mut_reverse(
        &mut self,
    ) -> impl ExactSizeIterator<Item = &'_ mut T> + FusedIterator<Item = &'_ mut T>
    {
        IterMut::<'_, T, Backward, N>::new({
            let mut c = self.cursor_mut();
            c.to_end();
            c
        })
    }
}

/// Advances a cursor in a direction
trait Advance {
    /// Try to advance the cursor in a direction and return, if it was
    fn try_advance<'a, T, const N: usize>(c: &mut CursorMut<'a, T, N>) -> bool
    where
        T: Sized + 'static;
}

/// Advances the cursor forward
struct Forward;

impl Advance for Forward {
    fn try_advance<'a, T, const N: usize>(c: &mut CursorMut<'a, T, N>) -> bool
    where
        T: Sized + 'static,
    {
        c.next()
    }
}

/// Advance the cursor backward
struct Backward;

impl Advance for Backward {
    fn try_advance<'a, T, const N: usize>(c: &mut CursorMut<'a, T, N>) -> bool
    where
        T: Sized + 'static,
    {
        c.previous()
    }
}

/// Forward iterator for cursors
struct IterMut<'a, T, A, const N: usize>
where
    T: Sized + 'static,
    A: Advance,
{
    visited_first: bool,
    cursor: CursorMut<'a, T, N>,
    pd: PhantomData<A>,
}

impl<'a, T, A, const N: usize> IterMut<'a, T, A, N>
where
    T: Sized + 'static,
    A: Advance,
{
    fn new(c: CursorMut<'a, T, N>) -> Self {
        Self {
            visited_first: false,
            cursor: c,
            pd: PhantomData,
        }
    }
}

impl<'a, T, A, const N: usize> Iterator for IterMut<'a, T, A, N>
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
            if !self.cursor.next() {
                return None;
            }
        }

        self.cursor.value()
    }
}

impl<'a, T, A, const N: usize> ExactSizeIterator for IterMut<'a, T, A, N>
where
    T: Sized + 'static,
    A: Advance,
{
    #[inline]
    fn len(&self) -> usize {
        self.cursor.list.len()
    }
}

impl<'a, T, A, const N: usize> FusedIterator for IterMut<'a, T, A, N>
where
    T: Sized + 'static,
    A: Advance,
{
}

impl<T, const N: usize> FromIterator<T> for LinkedList<T, N>
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
