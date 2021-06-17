use super::{
    node::{Node, NodeRef, NullNodeRef},
    LinkedList,
};
use std::ptr::null_mut;

/// Enables safe linked list iteration and modification
pub struct CursorMut<'a, T, const N: usize>
where
    T: Sized,
{
    /// Node the cursor is currently at. Guaranteed to never be null.
    node: *mut Node<T, N>,

    /// Current cursor position in the Node
    position: usize,

    /// Parent list
    pub(super) list: &'a mut LinkedList<T, N>,
}

impl<'a, T, const N: usize> CursorMut<'a, T, N>
where
    T: Sized + 'static,
{
    /// Create a cursor over the passed list, setting the cursor position to the
    /// passed node.
    /// Node must not be null.
    /// Position is ignored, if node is empty.
    #[inline]
    pub(super) unsafe fn new(
        list: &'a mut LinkedList<T, N>,
        node: *mut Node<T, N>,
        position: usize,
    ) -> Self {
        Self {
            list,
            position,
            node,
        }
    }

    /// Shorthand for accessing the current Node as a reference
    #[inline]
    fn node(&mut self) -> &'a mut Node<T, N> {
        unsafe { &mut *self.node }
    }

    /// Tries to advances cursor to the next position.
    /// Returns false, if there is no next position and the cursor did not
    /// advance.
    #[inline]
    pub fn next(&mut self) -> bool {
        if self.position + 1 < self.node().len() {
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
    #[inline]
    pub fn previous(&mut self) -> bool {
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

    /// Navigate to the start of the linked list
    #[inline]
    pub fn to_start(&mut self) {
        self.node = self.list.head;
    }

    /// Navigate to the end of the linked list
    #[inline]
    pub fn to_end(&mut self) {
        // `self.node().len() -1` can be negative only in case of an empty list
        if self.list.length == 0 {
            self.to_start();
        } else {
            // In all other cases a node can not be empty
            self.node = self.list.tail;
            self.position = self.node().len() - 1;
        }
    }

    /// Returns a reference to the current position's value.
    /// Only returns None, if the current linked list is empty.
    #[inline]
    pub fn value(&mut self) -> Option<&'a mut T> {
        if self.node().len() == 0 {
            None
        } else {
            Some(self.node().value(self.position))
        }
    }

    /// Returns a reference to the current value, that can be stored and used to
    /// construct cursors.
    #[inline]
    pub fn reference(&mut self) -> Option<NodeRef<T, N>> {
        if self.node().len() == 0 {
            None
        } else {
            Some(Node::reference(self.node, self.position))
        }
    }

    /// Insert value before the current cursor position
    pub fn insert_before(&mut self, val: T) {
        self.list.length += 1;

        if self.node().len() == 0 {
            self.node().append(val);
            self.position = 0;
            return;
        }

        // Append to previous node
        if self.position == 0 && self.node().len() == N {
            let new = self.node().append_to_previous(val);
            if self.list.head == self.node {
                self.list.head = new;
            }
            return;
        }

        // Insert into current node and possibly split it
        let new = self.node().insert(self.position, val);
        if new != null_mut() {
            // Advance back to next value to maintain API consistency
            self.next();

            if self.list.tail == self.node {
                self.list.tail = new;
            }
        }
    }

    /// Insert value before the current cursor position
    pub fn insert_after(&mut self, val: T) {
        self.list.length += 1;

        if self.node().len() == 0 {
            self.node().append(val);
            self.position = 0;
            return;
        }

        let new = self.node().insert(self.position + 1, val);
        if new != null_mut() && self.list.tail == self.node {
            self.list.tail = new;
        }
    }

    /// Remove current value, if any.
    ///
    /// Returns the removed value and a reference to the removed value, if one
    /// was ever taken for it.
    ///
    /// Sets the cursor to the previous node. If none, sets it to the next node.
    ///
    /// # Safety
    ///
    /// Removing a value will invalidate any NodeRef pointing to it. It is the
    /// caller's responsibility to remove any NodeRef to a removed value.
    pub unsafe fn remove(&mut self) -> Option<(T, Option<NullNodeRef<T, N>>)> {
        if self.list.len() == 0 {
            return None;
        }
        self.list.length -= 1;

        // Navigate to the previous or next sibling ahead of time.
        // Save node pointer, in case node is to be removed.
        let to_remove = (self.node, self.position);
        let removing_node = self.list.len() != 1 && self.node().len() == 1;
        if !self.previous() {
            self.next();
        }

        let re = Node::remove(to_remove.0, to_remove.1);

        if removing_node {
            if self.list.head == to_remove.0 {
                // Was head, so we moved to the next node
                self.list.head = self.node;
            }
            if self.list.tail == to_remove.0 {
                // Was tail, so we moved to the previous node
                self.list.tail = self.node;
            }
        }

        Some(re)
    }
}
