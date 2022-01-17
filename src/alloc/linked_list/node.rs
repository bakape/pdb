use super::{cursor::CursorMut, LinkedList};
use std::ptr::{null_mut, NonNull};

// TODO: use single element pointer stable nodes instead and just keep the
// ability to save the last cursor position

/// [LinkedList] list node containing up to `N` values of type `T`.
pub(super) struct Node<T>
where
    T: Sized,
{
    /// Previous node in the list
    pub(super) previous: *mut Node<T>,

    /// Next node in the list
    pub(super) next: *mut Node<T>,

    /// Contained value
    pub(super) value: T,
}

impl<T> Node<T>
where
    T: Sized,
{
    /// Creates new node pointer containing the `val`
    pub(super) fn new(val: T) -> NonNull<Self> {
        Box::into_raw(Box::new(Self {
            value: val,
            next: null_mut(),
            previous: null_mut(),
        }))
        .into()
    }

    /// Convert self to raw pointer
    #[inline]
    fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    /// Set the previous [Node] pointer and set the next [Node] pointer of the
    /// previous [Node], if any
    #[inline]
    pub(super) fn set_previous(&mut self, previous: *mut Self) {
        self.previous = previous;
        if previous != null_mut() {
            unsafe {
                (*previous).next = self as *mut _;
            }
        }
    }

    /// Set the next [Node] pointer and set the previous [Node] pointer of the
    /// next [Node], if any
    #[inline]
    pub(super) fn set_next(&mut self, next: *mut Self) {
        self.next = next;
        if next != null_mut() {
            unsafe {
                (*next).previous = self as *mut _;
            }
        }
    }

    /// Drop the [Node] and all the [Node]s after it in the list
    pub(super) fn drop_list(self) {
        let mut next = self.next;
        while next != null_mut() {
            let b = unsafe { Box::from_raw(next) };
            next = b.next;
        }
    }

    // /// Return reference to the next [Node], if any
    // pub fn previous(&self) -> Option<> {
    //     unsafe { self.previous.as_mut() }.map(|n| NodeCursor {
    //         node: n.into(),
    //         position: (N
    //             - 1
    //             - n.references
    //                 .iter()
    //                 .rev()
    //                 .position(|l| !l.is_null())
    //                 .unwrap()),
    //     })
    // }

    // /// Return cursor to the first position of the previous [Node], if any
    // pub fn next(&self) -> Option<NodeCursor<T, N>> {
    //     unsafe { self.next.as_mut() }.map(|n| NodeCursor {
    //         node: n.into(),
    //         position: n.references.iter().position(|l| !l.is_null()).unwrap(),
    //     })
    // }

    // /// Shift `n` values in the region `[start; start + n)` `shift` positions.
    // /// A negative `shift` shifts to the left and a positive `shift` shifts to
    // /// the right.
    // ///
    // /// # Panics
    // ///
    // /// Panics, if either `start + shift` or `start + n + shift` are out of
    // /// bounds.
    // fn shift(&mut self, start: usize, n: usize, shift: isize) {
    //     let new_start = (start as isize + shift) as usize;
    //     unsafe {
    //         copy(
    //             self.values[start..].as_mut_ptr(),
    //             self.values[new_start..].as_mut_ptr(),
    //             n,
    //         );
    //         copy(
    //             self.references[start..].as_mut_ptr(),
    //             self.references[new_start..].as_mut_ptr(),
    //             n,
    //         );
    //     }
    //     for l in self.references[new_start..new_start + n].iter_mut() {
    //         if !l.is_null() {
    //             unsafe {
    //                 (**l).position = ((**l).position as isize + shift) as usize;
    //             }
    //         }
    //     }
    // }

    // /// Appends a value to the [Node] and and returns a [Ref] to the value.
    // ///
    // /// # Panics
    // ////
    // /// Panics, if node capacity is exceeded.
    // #[inline]
    // pub fn append(&mut self, val: T) -> Ref<T, N> {
    //     let loc = Location::new(self, self.end);
    //     self.values[self.end as usize] = MaybeUninit::new((val, loc));
    //     self.end += 1;
    //     loc.into()
    // }

    // /// Appends a value to the previous [Node] and returns a [Ref] to the
    // /// value.
    // ///
    // /// If the previous [Node] is is full or not set, a new [Node] is created
    // /// and  returned.
    // pub fn append_to_previous(&mut self, val: T) -> (*mut Self, Ref<T, N>) {
    //     match unsafe { self.previous().as_mut() } {
    //         None => {
    //             let re = Node::new(val);
    //             self.set_previous(re.0);
    //             re
    //         }
    //         Some(prev) if prev.len() == N as u8 => {
    //             let re = Node::new(val);
    //             prev.set_next(re.0);
    //             self.set_previous(re.0);
    //             re
    //         }
    //         Some(prev) => (null_mut(), prev.append(val)),
    //     }
    // }

    // /// Push value to the start of the next [Node] and returns a [Ref] to
    // /// the value.
    // //
    // /// If the next [Node] is is full or not set, a new [Node] is created and
    // /// returned.
    // pub fn prepend_to_next(&mut self, val: T) -> (*mut Self, Ref<T, N>) {
    //     match unsafe { self.next().as_mut() } {
    //         None => {
    //             let re = Node::new(val);
    //             self.set_next(re.0);
    //             re
    //         }
    //         Some(next) if next.len() == N as u8 => {
    //             let re = Node::new(val);
    //             next.set_previous(re.0);
    //             self.set_next(re.0);
    //             re
    //         }
    //         Some(next) => (null_mut(), next.insert_non_full(0, val)),
    //     }
    // }

    // /// Insert value into the passed position in the [Node], shifting all
    // /// following values to the right and returning a [Ref] to the value.
    // /// If a new next [Node] is created containing overflown shifted values, it
    // /// is returned.
    // ///
    // /// # Panics
    // ///
    // /// Panics, if insertion would result in a sparse array.
    // pub fn insert(&mut self, i: u8, val: T) -> (*mut Self, Ref<T, N>) {
    //     if self.len() < N as u8 {
    //         return (null_mut(), self.insert_non_full(i, val));
    //     }

    //     // Split the current array by moving all following values to a new node
    //     let new_node = Node::empty();
    //     let new_node_len = self.end - i;
    //     unsafe {
    //         copy_nonoverlapping(
    //             self.values[i as usize..].as_ptr(),
    //             (*new_node).values.as_mut_ptr(),
    //             new_node_len as usize,
    //         );
    //         (*new_node).end = new_node_len;

    //         for (i, (_, loc)) in (*new_node).iter_mut().enumerate() {
    //             (**loc).node = new_node;
    //             (**loc).position = i as u8;
    //         }
    //     }

    //     let loc = Location::new(self, i);
    //     self.values[i as usize] = MaybeUninit::new((val, loc));
    //     self.end = i + 1;

    //     if self.next != null_mut() {
    //         unsafe {
    //             (*self.next).set_previous(new_node);
    //         }
    //     }
    //     self.set_next(new_node);

    //     (new_node, loc.into())
    // }

    // /// Insert value into non-full [Node] at position `i`, returning a [Ref]
    // /// to the value.
    // ///
    // /// # Panics
    // ///
    // /// Panics, if insertion would result in a sparse array or is out of bounds.
    // fn insert_non_full(&mut self, i: u8, val: T) -> Ref<T, N> {
    //     let loc = Location::new(self, i + self.start);
    //     let new_val = MaybeUninit::new((val, loc));
    //     let reference: Ref<T, N> = loc.into();

    //     if i == 0 && self.start != 0 {
    //         // Prepend in free space at the start of the array
    //         self.start -= 1;
    //         self.values[self.start as usize] = new_val;
    //         unsafe {
    //             (*loc).position = self.start;
    //         }
    //         reference
    //     } else if i + self.start == self.end {
    //         // Append as last value
    //         self.values[self.end as usize] = new_val;
    //         self.end += 1;
    //         reference
    //     } else {
    //         assert!(
    //             i + self.start <= self.end,
    //             "value insertion would result in sparse array"
    //         );

    //         // See shifting to which side is cheaper
    //         let shift_left =
    //             self.start != 0 && i <= (self.end - self.start) / 2;
    //         let i = (self.start + i) as usize;
    //         if shift_left {
    //             // Shift all preceding values to the left
    //             unsafe {
    //                 copy(
    //                     self.values[i].as_mut_ptr(),
    //                     self.values[i - 1].as_mut_ptr(),
    //                     i,
    //                 );
    //             }
    //             self.start -= 1;
    //             for (_, loc) in self.iter_mut().take(i) {
    //                 unsafe {
    //                     (**loc).position -= 1;
    //                 }
    //             }
    //         } else {
    //             // Shift all following values to the right
    //             unsafe {
    //                 copy(
    //                     self.values[i].as_mut_ptr(),
    //                     self.values[i + 1].as_mut_ptr(),
    //                     self.end as usize - i,
    //                 );
    //             }
    //             self.end += 1;
    //             for (_, loc) in self.iter_mut().skip(i + 1) {
    //                 unsafe {
    //                     (**loc).position += 1;
    //                 }
    //             }
    //         }

    //         reference
    //     }
    // }

    // /// Remove value at position `i`.
    // /// Returns the removed value, a [NullRef] to the removed value's
    // /// position before removal and, if the [Node] itself was removed.
    // ///
    // /// Empty [Node]s with either a previous or next [Node] are removed.
    // /// A [Node] that has neither a previous nor next node will never be removed.
    // ///
    // /// # Panics
    // ///
    // /// Panics, if `i` is out of bounds.
    // ///
    // /// # Safety
    // ///
    // /// Removing a value will invalidate any [Ref] pointing to it. It is the
    // /// caller's responsibility to remove any [Ref]s to a removed [Node].
    // ///
    // /// A removed [Node] is deallocated by this function. The caller should not
    // /// access it anymore.
    // //
    // // TODO: make all nodes removable
    // pub unsafe fn remove(
    //     node: *mut Self,
    //     mut i: u8,
    // ) -> (T, NullRef<T, N>, bool) {
    //     let this = &mut *node;
    //     i += this.start;
    //     assert!(i < this.end, "value removal out of bounds");

    //     let (val, loc) = {
    //         let mut tuple = MaybeUninit::uninit();
    //         copy_nonoverlapping(
    //             this.values[i as usize].as_ptr(),
    //             tuple.as_mut_ptr(),
    //             1,
    //         );
    //         let (val, loc) = tuple.assume_init();
    //         (val, loc.into())
    //     };

    //     if this.len() == 1 {
    //         // Ensure only the first node in an empty list can have zero
    //         // length
    //         if this.previous == null_mut() && this.next == null_mut() {
    //             this.end = 0;
    //         } else {
    //             if this.previous != null_mut() {
    //                 (*this.previous).set_previous(this.next);
    //             } else {
    //                 // This node was the head
    //                 (*this.next).previous = null_mut();
    //             }
    //             node.drop_in_place();
    //             return (val, loc, true);
    //         }
    //     } else if i == this.start {
    //         // Cheaply invalidate the first value
    //         this.start += 1;
    //     } else if i == this.end - 1 {
    //         // Cheaply invalidate the last value
    //         this.end -= 1;
    //     } else {
    //         // See shifting which side is cheaper
    //         if i - this.start <= this.end - i {
    //             // Shift all preceding values to the right
    //             let start = this.start as usize;
    //             let copying = i as usize - start;
    //             copy(
    //                 this.values[start].as_mut_ptr(),
    //                 this.values[start + 1].as_mut_ptr(),
    //                 copying,
    //             );
    //             this.start += 1;
    //             for (_, loc) in this.iter_mut().take(copying) {
    //                 (**loc).position += 1;
    //             }
    //         } else {
    //             // Shift all following values to the left
    //             let start = i as usize;
    //             let copying = this.end as usize - start;
    //             copy(
    //                 this.values[start + 1].as_mut_ptr(),
    //                 this.values[start].as_mut_ptr(),
    //                 copying,
    //             );
    //             this.end -= 1;
    //             for (_, loc) in this.iter_mut().rev().take(copying) {
    //                 (**loc).position -= 1;
    //             }
    //         }
    //     }

    //     (val, loc, false)
    // }

    // /// Create iterator over the [Node]'s value-reference pairs
    // #[inline]
    // fn iter_mut(
    //     &mut self,
    // ) -> impl Iterator<Item = &'_ mut (T, *mut Location<T, N>)> + DoubleEndedIterator
    // {
    //     self.values[self.start as usize..self.end as usize]
    //         .iter_mut()
    //         .map(|p| unsafe { &mut *p.as_mut_ptr() })
    // }
}

/// Storable reference to a [Node]
#[derive(Eq, Clone)]
pub struct Ref<T>(*mut Node<T>)
where
    T: Sized;

impl<T> Ref<T>
where
    T: Sized,
{
    // TODO: immutable cursor

    /// Obtain a mutable cursor to the referenced [Node].
    ///
    /// # Safety
    ///
    /// This method is only safe to call with the same [LinkedList] that the
    /// [Ref] was obtained from, and only if the [Node] has not been removed
    /// from the list yet.
    /// It is the caller's responsibility to remove any [Ref] to a removed
    /// [Node].
    #[inline]
    pub unsafe fn cursor_mut<'a>(
        &self,
        list: &'a mut LinkedList<T>,
    ) -> CursorMut<'a, T> {
        CursorMut::new(list, self.0)
    }
}

impl<T> From<*mut Node<T>> for Ref<T>
where
    T: Sized,
{
    #[inline]
    fn from(n: *mut Node<T>) -> Self {
        Self(n)
    }
}

impl<T> PartialEq for Ref<T>
where
    T: Sized,
{
    #[inline]
    fn eq(&self, other: &Ref<T>) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl<T> PartialEq<NullRef<T>> for Ref<T>
where
    T: Sized,
{
    #[inline]
    fn eq(&self, other: &NullRef<T>) -> bool {
        self == &other.0
    }
}

/// Reference to a removed node value. Can be used for equality comparison with
/// [Ref].
///
/// [NullRef] must be used to remove any stored [Ref] before any new
/// [Node] is inserted, because there is small but non-zero chance, that a new
/// [Node] will contain the same pointer as a previous [Node] and thus be
/// considered equal.
#[derive(Clone)]
pub struct NullRef<T>(Ref<T>)
where
    T: Sized;

impl<T> From<*mut Node<T>> for NullRef<T>
where
    T: Sized,
{
    #[inline]
    fn from(n: *mut Node<T>) -> Self {
        Self(n.into())
    }
}
