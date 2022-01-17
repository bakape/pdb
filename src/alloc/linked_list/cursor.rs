use super::{
    node::{Node, NullRef, Ref},
    LinkedList,
};
use std::ptr::{null_mut, NonNull};

// TODO: Immutable cursor

/// Common functionality for both [Cursor] and [CursorMut]
struct Common<T, L>
where
    T: Sized,
    L: VisitRef<LinkedList<T>>,
{
    /// Current cursor position.
    /// /Can only be null, if parent [LinkedList] is empty.
    node: *mut Node<T>,

    /// Parent [LinkedList]
    list: L,
}

impl<T, L> Common<T, L>
where
    T: Sized,
    L: VisitRef<LinkedList<T>>,
{
    /// Tries to advances cursor to the next position.
    /// Returns false, if there is no next position and the cursor did not
    /// advance.
    #[inline] // To avoid function call overhead on iteration
    pub fn next(&mut self) -> bool {
        if self.node == null_mut() {
            return false;
        }

        let next = unsafe { (*self.node).next };
        if next != null_mut() {
            self.node = next;
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
        if self.node == null_mut() {
            return false;
        }

        let prev = unsafe { (*self.node).previous };
        if prev != null_mut() {
            self.node = prev;
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
        self.node = self.list.with(|ll| ll.tail);
    }

    /// Returns a reference to the current value, that can be stored and used to
    /// construct cursors.
    ///
    /// Only returns [None], if the [LinkedList] is empty.
    #[inline]
    fn reference(&self) -> Option<Ref<T>> {
        unsafe { self.node.as_mut() }.map(|n| n.into())
    }
}

/// Allows accessing self as a [LinkedList] reference
trait VisitRef<T> {
    /// Runs a visitor function on the linked list
    fn with<R>(&self, visit: impl FnOnce(&T) -> R) -> R;
}

impl<'l, T> VisitRef<LinkedList<T>> for &'l LinkedList<T>
where
    T: Sized,
{
    #[inline]
    fn with<R>(&self, visit: impl FnOnce(&LinkedList<T>) -> R) -> R {
        visit(self)
    }
}

impl<'l, T> VisitRef<LinkedList<T>> for &'l mut LinkedList<T>
where
    T: Sized,
{
    #[inline]
    fn with<R>(&self, visit: impl FnOnce(&LinkedList<T>) -> R) -> R {
        visit(self)
    }
}

/// Enables safe linked list iteration and modification
pub struct CursorMut<'l, T>
where
    T: Sized,
{
    common: Common<T, &'l mut LinkedList<T>>,
}

impl<'l, T> CursorMut<'l, T>
where
    T: Sized + 'static,
{
    /// Create a cursor over the passed list, setting the cursor position to the
    /// passed `node`. `node` must not be null.
    #[inline]
    pub(super) unsafe fn new(
        list: &'l mut LinkedList<T>,
        node: *mut Node<T>,
    ) -> Self {
        Self {
            common: Common { node, list },
        }
    }
}

impl<'c, 'l: 'c, T> CursorMut<'l, T>
where
    T: Sized + 'static,
{
    /// Returns a reference to the parent list
    #[inline]
    pub(super) fn list(&self) -> &LinkedList<T> {
        self.common.list
    }

    /// Returns a mutable reference to the parent list
    #[cfg(test)]
    pub(super) fn list_mut(&mut self) -> &mut LinkedList<T> {
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

    /// Returns a reference to the current node's value.
    /// Only returns [None], if the [LinkedList] is empty.
    #[inline]
    pub fn value(&'c mut self) -> Option<&'l mut T> {
        unsafe { self.common.node.as_mut() }.map(|n| n.value)
    }

    /// Returns a [Ref] to the current node, that can be stored and used to
    /// construct cursors.
    ///
    /// Only returns [None], if the [LinkedList] is empty.
    pub fn reference(&self) -> Option<Ref<T>> {
        self.common.reference()
    }

    /// Insert value before the current cursor position, returning a [Ref]
    /// to the inserted value.
    ///
    /// If the [LinkedList] is empty prior to this call, the cursor is navigated
    /// to the inserted node.
    pub fn insert_before(&mut self, val: T) -> Ref<T> {
        self.common.list.length += 1;
        let n = Node::new(val);

        if self.common.node == null_mut() {
            return self.insert_only(n);
        }

        let prev = unsafe { (*self.common.node).previous };
        if prev == null_mut() {
            self.common.list.head = n;
        }
        unsafe { *self.common.node }.set_previous(n);
        n.into()
    }

    /// Insert value before the current cursor position, returning a [Ref]  to
    /// the inserted value.
    ///
    /// If the [LinkedList] is empty prior to this call, the cursor is navigated
    /// to the inserted node.
    pub fn insert_after(&mut self, val: T) -> Ref<T> {
        self.common.list.length += 1;
        let n = Node::new(val);

        if self.common.node == null_mut() {
            return self.insert_only(n);
        }

        let next = unsafe { *(self.common.node).next };
        if next == null_mut() {
            self.common.list.tail = n;
        }
        unsafe { *self.common.node }.set_next(n);
        n.into()
    }

    /// Insert node and set it as the head and tail, returning a [Ref] to it
    fn insert_only(&mut self, n: NonNull<Node<T>>) -> Ref<T> {
        self.common.node =
            self.common.list.head = self.common.list.tail = n.into();
        n.into()
    }

    /// Remove current node, if any.
    /// Returns the removed value and a reference to the removed node.
    /// Only returns [None], if the [LinkedList] is empty.
    ///
    /// Sets the cursor to the previous node. If none, sets it to the next
    /// node.
    ///
    /// # Safety
    ///
    /// Removing a node will invalidate any [Ref] pointing to it. It is the
    /// caller's responsibility to remove any [Ref] to a removed node.
    pub unsafe fn remove(&mut self) -> Option<(T, NullRef<T>)> {
        if self.common.node == null_mut() {
            return None;
        }

        self.common.list.length -= 1;
        if self.common.list.head == self.common.node {
            self.common.list.head = unsafe { (*self.common.node).next };
        }
        if self.common.list.tail == self.common.node {
            self.common.list.tail = unsafe { (*self.common.node).previous };
        }

        let r = NullRef::from(self.common.node);
        let cur = unsafe { Box::from_raw(self.common.node) };
        if cur.previous != null_mut() {
            unsafe { *cur.previous }.set_next(cur.next);
            self.common.node = cur.previous;
        } else {
            if cur.next != null_mut() {
                unsafe { *cur.next }.set_previous(null_mut());
            }
            self.common.node = cur.next;
        }
        Some(cur.value, r)
    }
}
