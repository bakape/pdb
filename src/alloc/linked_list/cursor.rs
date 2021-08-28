use super::{
    node::{Node, NodeCursor, NullRef, Ref},
    LinkedList,
};
use std::ptr::null_mut;

// TODO: Immutable cursor

/// Common functionality for both [Cursor] and [CursorMut]
struct Common<T, L, const N: usize>
where
    T: Sized,
    L: VisitRef<LinkedList<T, N>>,
{
    /// Cursor over the current [Node]. Only [None], if list is empty.
    cursor: Option<NodeCursor<T, N>>,

    /// Parent [LinkedList]
    list: L,
}

impl<T, L, const N: usize> Common<T, L, N>
where
    T: Sized,
    L: VisitRef<LinkedList<T, N>>,
{
    /// Tries to advances cursor to the next position.
    /// Returns false, if there is no next position and the cursor did not
    /// advance.
    #[inline] // To avoid function call overhead on iteration
    fn next(&mut self) -> bool {
        if self.position as usize + 1 < self.node().len() as usize {
            self.position += 1;
            true
        } else if self.node().next() != null_mut() {
            // Next node can not have zero length
            self.node = self.node().next();
            self.position = 0;
            true
        } else {
            false
        }
    }

    /// Tries to move cursor to the previous position.
    /// Returns false, if there is no previous position and the cursor did not
    /// move.
    #[inline] // To avoid function call overhead on iteration
    fn previous(&mut self) -> bool {
        if self.position != 0 {
            self.position -= 1;
            return true;
        }

        let prev = self.node().previous();
        if prev != null_mut() {
            self.node = prev;
            self.position = self.node().len() - 1;
            true
        } else {
            false
        }
    }

    /// Navigate to the start of the [LinkedList]
    #[inline]
    fn seek_to_start(&mut self) {
        self.node = self.list.with(|ll| ll.head);
    }

    /// Navigate to the end of the [LinkedList]
    #[inline]
    fn seek_to_end(&mut self) {
        // `self.node().len() -1` can be negative only in case of an empty list
        if self.list.with(|ll| ll.length) == 0 {
            self.seek_to_start();
        } else {
            // In all other cases a node can not be empty
            self.node = self.list.with(|ll| ll.tail);
            self.position = self.node().len() - 1;
        }
    }

    /// Returns a reference to the current value, that can be stored and used to
    /// construct cursors.
    ///
    /// Only returns [None], if the current linked list is empty.
    #[inline]
    fn reference(&self) -> Option<Ref<T, N>> {
        if self.node().len() == 0 {
            None
        } else {
            Some(self.node().reference(self.position))
        }
    }
}

/// Allows accessing self as a [LinkedList] reference
trait VisitRef<T> {
    /// Runs a visitor function on the linked list
    fn with<R>(&self, visit: impl FnOnce(&T) -> R) -> R;
}

impl<'l, T, const N: usize> VisitRef<LinkedList<T, N>> for &'l LinkedList<T, N>
where
    T: Sized,
{
    #[inline]
    fn with<R>(&self, visit: impl FnOnce(&LinkedList<T, N>) -> R) -> R {
        visit(self)
    }
}

impl<'l, T, const N: usize> VisitRef<LinkedList<T, N>>
    for &'l mut LinkedList<T, N>
where
    T: Sized,
{
    #[inline]
    fn with<R>(&self, visit: impl FnOnce(&LinkedList<T, N>) -> R) -> R {
        visit(self)
    }
}

/// Enables safe linked list iteration and modification
pub struct CursorMut<'l, T, const N: usize>
where
    T: Sized,
{
    common: Common<T, &'l mut LinkedList<T, N>, N>,
}

impl<'l, T, const N: usize> CursorMut<'l, T, N>
where
    T: Sized + 'static,
{
    /// Create a cursor over the passed list, setting the cursor position to the
    /// passed node.
    /// `node` must not be null.
    /// `position` is ignored, if node is empty.
    #[inline]
    pub(super) unsafe fn new(
        list: &'l mut LinkedList<T, N>,
        node: *mut Node<T, N>,
        position: usize,
    ) -> Self {
        Self {
            common: Common {
                node,
                position,
                list,
            },
        }
    }
}

impl<'c, 'l: 'c, T, const N: usize> CursorMut<'l, T, N>
where
    T: Sized + 'static,
{
    /// Returns a reference to the parent list
    #[inline]
    pub(super) fn list(&self) -> &LinkedList<T, N> {
        self.common.list
    }

    /// Returns a mutable reference to the parent list
    #[cfg(test)]
    pub(super) fn list_mut(&mut self) -> &mut LinkedList<T, N> {
        self.common.list
    }

    /// Tries to advances cursor to the next position.
    /// Returns false, if there is no next position and the cursor did not
    /// advance.
    #[inline] // To avoid function call overhead on iteration
    pub fn next(&mut self) -> bool {
        self.common.next()
    }

    /// Tries to move cursor to the previous position.
    /// Returns false, if there is no previous position and the cursor did not
    /// move.
    #[inline] // To avoid function call overhead on iteration
    pub fn previous(&mut self) -> bool {
        self.common.previous()
    }

    /// Navigate to the start of the [LinkedList]
    pub fn seek_to_start(&mut self) {
        self.common.seek_to_start()
    }

    /// Navigate to the end of the [LinkedList]
    pub fn seek_to_end(&mut self) {
        self.common.seek_to_end()
    }

    // TODO: move current value to the start or end of the list

    /// Returns a reference to the current position's value.
    /// Only returns None, if the current linked list is empty.
    #[inline]
    pub fn value(&'c mut self) -> Option<&'l mut T> {
        if self.node_mut().len() == 0 {
            None
        } else {
            Some(self.node_mut().value_mut(self.common.position))
        }
    }

    /// Returns a reference to the current value, that can be stored and used to
    /// construct cursors.
    ///
    /// Only returns [None], if the current linked list is empty.
    pub fn reference(&self) -> Option<Ref<T, N>> {
        self.common.reference()
    }

    /// Shorthand for accessing the current [Node] as a mutable reference
    #[inline]
    fn node_mut(&self) -> &mut Node<T, N> {
        unsafe { &mut *self.common.node }
    }

    /// Insert value before the current cursor position, returning a [Ref]
    /// to the inserted value
    pub fn insert_before(&mut self, val: T) -> Ref<T, N> {
        self.common.list.length += 1;

        let len = self.common.node().len();
        if len == 0 {
            self.common.position = 0;
            self.node_mut().append(val)
        } else if self.common.position == 0 && len == N as u8 {
            // Append to previous node
            let (new_node, r) = self.node_mut().append_to_previous(val);
            if self.common.list.head == self.common.node {
                self.common.list.head = new_node;
            }
            r
        } else {
            // Insert into current node and possibly split it
            let (new_node, r) =
                self.node_mut().insert(self.common.position, val);
            if new_node != null_mut()
                && self.common.list.tail == self.common.node
            {
                self.common.list.tail = new_node;
            }

            // Advance back to next value to maintain API consistency
            self.next();

            r
        }
    }

    /// Insert value before the current cursor position, returning a [Ref]  to
    /// the inserted value
    pub fn insert_after(&mut self, val: T) -> Ref<T, N> {
        self.common.list.length += 1;

        let len = self.common.node().len();
        if len == 0 {
            // Append to start of node
            self.common.position = 0;
            self.node_mut().append(val)
        } else if len == N as u8 && self.common.position == N as u8 - 1 {
            // Prepend to next node
            let (new_node, r) = self.node_mut().prepend_to_next(val);
            if new_node != null_mut()
                && self.common.node == self.common.list.tail
            {
                self.common.list.tail = new_node;
            }
            r
        } else {
            // Inserts into existing node, possibly splitting it
            let (new_node, r) =
                self.node_mut().insert(self.common.position + 1, val);
            if new_node != null_mut()
                && self.common.list.tail == self.common.node
            {
                self.common.list.tail = new_node;
            }
            r
        }
    }

    /// Remove current value, if any.
    /// Returns the removed value and a reference to the removed value.
    ///
    /// Sets the cursor to the previous [Node]. If none, sets it to the next
    /// [Node].
    ///
    /// # Safety
    ///
    /// Removing a value will invalidate any [Ref] pointing to it. It is the
    /// caller's responsibility to remove any [Ref] to a removed value.
    pub unsafe fn remove(&mut self) -> Option<(T, NullRef<T, N>)> {
        if self.common.list.len() == 0 {
            return None;
        }
        self.common.list.length -= 1;

        // Navigate to the previous or next sibling ahead of time.
        // Save node pointer, in case node is to be removed.
        let to_remove = (self.common.node, self.common.position);
        if !self.previous() {
            self.next();
        }

        let (val, reference, removed) = Node::remove(to_remove.0, to_remove.1);
        if removed {
            if self.common.list.head == to_remove.0 {
                // Was head, so we moved to the next node
                self.common.list.head = self.common.node;
            }
            if self.common.list.tail == to_remove.0 {
                // Was tail, so we moved to the previous node
                self.common.list.tail = self.common.node;
            }
        }
        Some((val, reference))
    }
}
