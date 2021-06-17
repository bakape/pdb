use super::{cursor::CursorMut, LinkedList};
use std::{intrinsics::copy_nonoverlapping, mem::MaybeUninit, ptr::null_mut};

/// Unrolled linked list node containing up to N values of type T.
/// N must fit into 7 bits as of 2021.
//
// TODO: validate N fits into 7 bits, when we have const generic constraints
pub(super) struct Node<T, const N: usize>
where
    T: Sized,
{
    /// Previous node in the list
    ///
    /// Also stores the used length of vals in the highest 3 bits of the
    /// pointer, that are not and will not be used for addressing for may years.
    /// This saves us 8 bytes because of struct padding.
    /// Most of the time you'd traverse the list from the front, so it's better
    /// to store it in the `previous` pointer, rather than the `next` one, to
    /// make getting the address of the next node slightly cheaper.
    previous: *mut Node<T, N>,

    /// Next node in the list
    next: *mut Node<T, N>,

    /// Array of values and optional references to these values
    vals: [MaybeUninit<(T, *mut Location<T, N>)>; N],
}

impl<T, const N: usize> Node<T, N>
where
    T: Sized,
{
    /// Number of highest bits in previous pointer used for length storage.
    ///
    /// Can use no more than 7 bits as of 2021.
    const LENGTH_BITS: usize = {
        let mut bits = 1;
        loop {
            bits += 1;
            if 1 << bits >= N {
                break;
            }
        }

        bits
    };

    /// Bits to shift a length value for
    const LENGTH_SHIFT: usize = {
        #[cfg(not(target_pointer_width = "64"))]
        compile_error!("only 64 bit systems are supported");

        64 - Self::LENGTH_BITS
    };

    /// Mask for resetting stored length
    const LENGTH_MASK: usize = {
        let mut i = 0;
        let mut mask = 0;
        while i < Self::LENGTH_BITS {
            mask |= 1 << 63 - i;
            i += 1;
        }
        !mask
    };

    /// Create new node pointer from value
    pub fn new(val: T) -> *mut Self {
        Self {
            vals: {
                let mut arr: [MaybeUninit<(T, *mut Location<T, N>)>; N] =
                    unsafe { MaybeUninit::uninit().assume_init() };
                arr[0] = Self::wrap_value(val);
                arr
            },
            next: null_mut(),
            previous: (0 | (1 << Self::LENGTH_SHIFT)) as *mut _,
        }
        .into_raw()
    }

    /// Create a new empty Node
    pub fn empty() -> *mut Self {
        Self {
            vals: unsafe { MaybeUninit::uninit().assume_init() },
            next: null_mut(),
            previous: null_mut(),
        }
        .into_raw()
    }

    /// Convert self to raw pointer
    #[inline]
    fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    /// Wrap value for inserting into the array
    #[inline]
    fn wrap_value(val: T) -> MaybeUninit<(T, *mut Location<T, N>)> {
        MaybeUninit::new((val, null_mut()))
    }

    /// Store the length of the array
    #[inline]
    fn set_length(&mut self, length: usize) {
        self.previous = (((self.previous as usize) & Self::LENGTH_MASK)
            | (length << Self::LENGTH_SHIFT)) as *mut _;
    }

    /// Return pointer to the previous node, if any
    #[inline]
    pub fn previous(&self) -> *mut Self {
        // Sign extend first to make the pointer canonical.
        // Note: Technically this is implementation defined. We may want a more
        // standard-compliant way to sign-extend the value.
        (((self.previous as usize) << Self::LENGTH_BITS) >> Self::LENGTH_BITS)
            as *mut _
    }

    /// Set the previous node pointer and set the next node pointer of the
    /// previous node, if any
    #[inline]
    fn set_previous(&mut self, previous: *mut Self) {
        debug_assert!(
            (previous as usize & Self::LENGTH_MASK) == previous as usize
        );

        self.store_previous(previous);
        if previous != null_mut() {
            unsafe {
                (*previous).next = self as *mut _;
            }
        }
    }

    /// Encodes the previous node pointer, without setting the next pointer on
    /// the previous node
    #[inline]
    fn store_previous(&mut self, previous: *mut Self) {
        let len = self.len() as usize;
        self.previous =
            (previous as usize | (len << Self::LENGTH_SHIFT)) as *mut _;
    }

    /// Return pointer to the next node, if any
    #[inline]
    pub fn next(&self) -> *mut Self {
        self.next
    }

    /// Set the next node pointer and set the previous node pointer of the next
    /// node, if any
    #[inline]
    fn set_next(&mut self, next: *mut Self) {
        self.next = next;
        if next != null_mut() {
            unsafe {
                (*next).store_previous(self as *mut _);
            }
        }
    }

    /// Return the occupied position count in the node
    #[inline]
    pub fn len(&self) -> usize {
        (self.previous as usize) >> Self::LENGTH_SHIFT
    }

    /// Drop the Node and all the nodes after it in the list
    pub fn drop_list(self) {
        let mut next = self.next;
        while next != null_mut() {
            let b = unsafe { Box::from_raw(next) };
            next = b.next;
        }
    }

    /// Returns a reference to the value-reference pair
    ///
    /// # Panics
    ///
    /// Panics, if index is out of bounds.
    #[inline]
    fn get(&mut self, i: usize) -> &'_ mut (T, *mut Location<T, N>) {
        let len = self.len();
        assert!(i < len, "index out of bounds");

        unsafe { &mut (*self.vals[i].as_mut_ptr()) }
    }

    /// Returns a reference to the value at position `i`.
    ///
    /// # Panics
    ///
    /// Panics, if index is out of bounds.
    #[inline]
    pub fn value<'a>(&mut self, i: usize) -> &'a mut T {
        unsafe { std::mem::transmute(&mut self.get(i).0) }
    }

    /// Returns a reference to the node's value at position `i`.
    /// `node must not be `null`.
    ///
    /// # Panics
    ///
    /// Panics, if index is out of bounds or `node` is `null`.
    #[inline]
    pub fn reference(node: *mut Self, i: usize) -> NodeRef<T, N> {
        assert!(node != null_mut());
        let t = unsafe { (*node).get(i) };

        if t.1 == null_mut() {
            t.1 = Box::into_raw(Location { node, position: i }.into());
        }
        NodeRef { location: t.1 }
    }

    /// Appends a value to the node.
    ///
    /// # Panics
    ////
    /// Panics, if node capacity is exceeded.
    #[inline]
    pub fn append(&mut self, val: T) {
        let l = self.len();
        self.vals[l] = Self::wrap_value(val);
        self.set_length(l + 1);
    }

    /// Appends a value to the previous node.
    ///
    /// If the previous node is is full or not set, a new node is created and
    /// returned.
    pub fn append_to_previous(&mut self, val: T) -> *mut Self {
        match unsafe { self.previous().as_mut() } {
            None => {
                let new = Node::new(val);
                self.set_previous(new);
                new
            }
            Some(prev) if prev.len() == N => {
                let new = Node::new(val);
                prev.set_next(new);
                self.set_previous(new);
                new
            }
            Some(prev) => {
                prev.append(val);
                null_mut()
            }
        }
    }

    /// Insert value into the passed position in the node, shifting all
    /// following nodes to the right.
    /// If a new next node is created containing overflown shifted values, it is
    /// returned.
    ///
    /// # Panics
    ///
    /// Panics, if insertion would result in a sparse array.
    pub fn insert(&mut self, i: usize, val: T) -> *mut Self {
        let len = self.len();

        if len < N {
            assert!(i <= len, "value insertion would result in sparse array");

            // Insert as last value
            let mut next = Self::wrap_value(val);
            if i == len {
                self.vals[i] = next;
                self.set_length(len + 1);
                return null_mut();
            }

            // Shift all following values
            let mut i = i;
            loop {
                std::mem::swap(&mut next, &mut self.vals[i]);
                unsafe {
                    let loc = (*next.as_mut_ptr()).1;
                    if loc != null_mut() {
                        (*loc).position += 1
                    }
                }
                i += 1;
                if i == len {
                    self.vals[i] = next;
                    self.set_length(len + 1);
                    return null_mut();
                }
            }
        }

        // Split the current array
        {
            let new = Node::empty();

            unsafe {
                let new_len = len - i;
                std::ptr::copy_nonoverlapping(
                    self.vals[i..].as_ptr(),
                    (*new).vals.as_mut_ptr(),
                    new_len,
                );
                (*new).set_length(new_len);

                for (i, (_, loc)) in (*new).iter_mut().enumerate() {
                    if *loc != null_mut() {
                        (**loc).node = new;
                        (**loc).position = i;
                    }
                }
            }

            self.vals[i] = Self::wrap_value(val);
            self.set_length(i + 1);

            if self.next != null_mut() {
                unsafe {
                    (*self.next).set_previous(new);
                }
            }
            self.set_next(new);

            return new;
        }
    }

    /// Remove value at position `i`.
    ///
    /// Returns the removed value and a reference to the removed value, if one
    /// was ever taken for it.
    ///
    /// Empty nodes with either a previous or next node are removed.
    /// A node that has neither a previous nor next node will never be removed.
    ///
    /// # Panics
    ///
    /// Panics, if `i` is out of bounds.
    ///
    /// # Safety
    ///
    /// Removing a value will invalidate any NodeRef pointing to it. It is the
    /// caller's responsibility to remove any NodeRef to a removed Node.
    pub unsafe fn remove(
        node: *mut Self,
        mut i: usize,
    ) -> (T, Option<NullNodeRef<T, N>>) {
        let this = &mut *node;

        let len = this.len();
        assert!(i < len, "value removal out of bounds");

        let mut tuple = MaybeUninit::uninit();
        copy_nonoverlapping(this.vals[i].as_ptr(), tuple.as_mut_ptr(), 1);
        let (val, loc) = tuple.assume_init();
        let loc = if loc == null_mut() {
            None
        } else {
            Some(loc.into())
        };

        if len == 1 {
            // Ensure only the first node in an empty list can have zero
            // length
            let prev = this.previous();
            if prev == null_mut() && this.next() == null_mut() {
                this.set_length(0);
            } else {
                if prev != null_mut() {
                    (*prev).set_previous(this.next);
                } else {
                    // This node was the head
                    (*this.next()).set_previous(null_mut());
                }
                node.drop_in_place();
            }

            (val, loc)
        } else {
            this.set_length(len - 1);

            // Shift all nodes to the left
            i += 1;
            while i < len {
                let loc = (*this.vals[i].as_mut_ptr()).1;
                if loc != null_mut() {
                    (*loc).position = i - 1;
                }

                copy_nonoverlapping(
                    this.vals[i].as_ptr(),
                    this.vals[i - 1].as_mut_ptr(),
                    1,
                );
                i += 1;
            }

            (val, loc)
        }
    }

    /// Create iterator over the node's value-reference pairs
    #[inline]
    fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &'_ mut (T, *mut Location<T, N>)> {
        let len = self.len();
        self.vals[..len]
            .iter_mut()
            .map(|p| unsafe { &mut *p.as_mut_ptr() })
    }
}

impl<T, const N: usize> Drop for Node<T, N>
where
    T: Sized,
{
    fn drop(&mut self) {
        let len = self.len();
        let mut i = 0;
        unsafe {
            while i < len {
                let mut tmp = MaybeUninit::uninit();
                copy_nonoverlapping(self.vals[i].as_ptr(), tmp.as_mut_ptr(), 1);

                let (_, loc) = tmp.assume_init(); // Drop value

                // Drop location
                if loc != null_mut() {
                    loc.drop_in_place();
                }

                i += 1;
            }
        }
    }
}

/// Describes the location of a value in a linked list
#[derive(Eq, PartialEq, Clone)]
struct Location<T, const N: usize>
where
    T: Sized,
{
    /// Parent Node
    node: *mut Node<T, N>,

    /// Position in the node
    position: usize,
}

/// Storable reference to a Node
#[derive(Eq, Clone)]
pub struct NodeRef<T, const N: usize>
where
    T: Sized,
{
    /// Pointer to location, that is updates as the value is moved around
    location: *mut Location<T, N>,
}

impl<T, const N: usize> NodeRef<T, N>
where
    T: Sized + 'static,
{
    /// Obtain a mutable cursor to the referenced Node.
    ///
    /// # Safety
    /// This method is only safe to call with the same list that the NodeRef was
    /// obtained from, and only if the Node has not been removed from the list.
    /// It is the caller's responsibility to remove any NodeRef to a removed
    /// Node.
    #[inline]
    pub unsafe fn cursor_mut<'a>(
        &self,
        list: &'a mut LinkedList<T, N>,
    ) -> CursorMut<'a, T, N> {
        CursorMut::new(list, (*self.location).node, (*self.location).position)
    }
}

impl<T, const N: usize> From<*mut Location<T, N>> for NodeRef<T, N>
where
    T: Sized,
{
    #[inline]
    fn from(loc: *mut Location<T, N>) -> Self {
        Self { location: loc }
    }
}

impl<T, const N: usize> PartialEq for NodeRef<T, N>
where
    T: Sized,
{
    #[inline]
    fn eq(&self, other: &NodeRef<T, N>) -> bool {
        self.location == other.location
    }
}

impl<T, const N: usize> PartialEq<NullNodeRef<T, N>> for NodeRef<T, N>
where
    T: Sized,
{
    #[inline]
    fn eq(&self, other: &NullNodeRef<T, N>) -> bool {
        self == &other.0
    }
}

/// Reference to a removed Node. Can be used for equality comparison with
/// NodeRef.
///
/// NullNodeRef must be used to remove any stored NodeRef before any new node is
/// inserted, because there is small but non-zero chance, that a new node
/// will contain the same pointers as a previous node and thus be considered
/// equal.
#[derive(Clone)]
pub struct NullNodeRef<T, const N: usize>(NodeRef<T, N>)
where
    T: Sized;

impl<T, const N: usize> From<*mut Location<T, N>> for NullNodeRef<T, N>
where
    T: Sized,
{
    #[inline]
    fn from(loc: *mut Location<T, N>) -> Self {
        Self(loc.into())
    }
}

impl<T, const N: usize> PartialEq<NodeRef<T, N>> for NullNodeRef<T, N>
where
    T: Sized,
{
    #[inline]
    fn eq(&self, other: &NodeRef<T, N>) -> bool {
        &self.0 == other
    }
}
