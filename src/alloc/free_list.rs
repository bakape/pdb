use super::linked_list::{LinkedList, NodeRef};

/// Range of memory in a buffer
#[derive(Clone, Eq, PartialEq)]
struct Range {
    /// Offset from page or segment start
    offset: usize,

    /// Size of range in bytes
    size: usize,
}

impl Range {
    /// Modifying the range to allocate a buffer in its start and return the
    // allocation's offset.
    /// The caller must ensure the range has more capacity than needed.
    fn allocate(&mut self, size: usize) -> usize {
        let off = self.offset;
        self.offset += size;
        self.size -= size;
        off
    }
}

/// Doubly linked list for keeping track of free memory ranges in a page
pub struct FreeList {
    /// Underlying free range linked list
    list: LinkedList<Range, 8>,

    /// Last inserted into free memory range
    last_used: Option<NodeRef<Range, 8>>,
}

/// Result of an `insert()` call to the FreeList
pub enum AllocationResult {
    /// Successfully allocated. Contains the offset of the allocation.
    Allocated(usize),

    /// Space large enough for the allocation not found. Contains the size of
    /// the largest free memory region encountered.
    NotFound(usize),
}

impl FreeList {
    /// Creates a new `FreeList` with the passed capacity
    pub fn new(cap: usize) -> Self {
        let mut ll = LinkedList::new();
        Self {
            last_used: {
                let mut c = ll.cursor_mut();
                c.insert_after(Range {
                    offset: 0,
                    size: cap,
                });
                c.reference()
            },
            list: ll,
        }
    }

    /// Pad size to ensure all free ranges are aligned
    fn pad_size(size: &mut usize) {
        const WORD: usize = std::mem::size_of::<usize>();
        *size += WORD - (*size % WORD);
    }

    /// Tries to register an insertion in the free list and returns the offset
    // to write the data to, if a space for it can be found.
    pub fn allocate(&mut self, mut size: usize) -> AllocationResult {
        // Using a first-fit algorithm. Expect faster lookup times to outweigh
        // the possible greater fragmentation.

        Self::pad_size(&mut size);

        // Hot path
        if let Some(reference) = &self.last_used {
            let mut c = unsafe { reference.cursor_mut(&mut self.list) };
            let range = c.value().unwrap();
            if range.size > size {
                // Still some space left in the range

                return AllocationResult::Allocated(range.allocate(size));
            } else if range.size == size {
                // Range depleted

                let offset = range.offset;

                // Upholds the safety contract
                self.last_used = None;
                unsafe { c.remove() };

                return AllocationResult::Allocated(offset);
            }
        }

        let mut c = self.list.cursor_mut();
        let mut max_size = 0;
        loop {
            let range = match c.value() {
                Some(r) => r,
                None => return AllocationResult::NotFound(0),
            };

            if max_size < range.size {
                max_size = range.size;
            }

            if range.size > size {
                // Still some space left in the range

                self.last_used = c.reference();
                return AllocationResult::Allocated(range.allocate(size));
            } else if range.size == size {
                // Range depleted

                let offset = range.offset;

                // Upholds the safety contract
                match (&self.last_used, &c.reference()) {
                    (Some(range), Some(reference)) if range.eq(reference) => {
                        self.last_used = None;
                    }
                    _ => (),
                }
                unsafe { c.remove() };

                return AllocationResult::Allocated(offset);
            }

            if !c.next() {
                return AllocationResult::NotFound(max_size);
            }
        }
    }

    /// Mark a memory region as free in the list
    pub fn free(
        &mut self,
        offset: usize,
        mut size: usize,
    ) -> Result<(), &'static str> {
        Self::pad_size(&mut size);

        let new = Range { offset, size };
        let mut c = self.list.cursor_mut();
        loop {
            match c.value() {
                Some(r) if offset < r.offset => {
                    c.insert_before(new);
                    return Ok(());
                }
                Some(r) if offset >= r.offset + r.size => {
                    if !c.next() {
                        // Add new range to the end of the list
                        c.insert_after(new);
                        return Ok(());
                    }
                }
                None => {
                    // No free regions, so add one
                    c.insert_before(Range { offset, size });
                    self.last_used = c.reference();
                    return Ok(());
                }
                _ => {
                    return Err("new range overlaps with existing range");
                }
            };
        }
    }
}
