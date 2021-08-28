mod free_list;
mod linked_list;

use self::free_list::FreeList;
use lazy_static::lazy_static;
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    ptr::null_mut,
    sync::{Arc, Mutex, RwLock},
    time::Instant,
    usize,
};

lazy_static! {
    /// Global Allocator instance
    static ref ALLOCATOR: Mutex<Allocator> = Default::default();

    // TODO: allow bumping usage time w/o allocator lock via a eventual
    // consistency updates:
    // - have global Mutex<HashMap<page_id, last_used_time>>
    // - insert into map on each page use
    // - take() and merge map into LRU max heap, when we need to look for a
    //   pages to swap
    // - when merging, filter currently returned pages
    // - when a page is retuned, remove it from the LRU HEAP
    // - bump LRU both on non-allocator lock acquisition and release to prevent
    //   the page from being swapped while in use
    //
}

/// Wraps a pointer to an allocated fixed size buffer with dropping and
// dereferencing to a slice
struct Buffer<const CAP: usize> {
    ptr: *mut u8,
}

impl<const CAP: usize> Buffer<CAP> {
    /// Allocates a new fixed size buffer
    fn new() -> Self {
        Self {
            ptr: unsafe { libc::malloc(CAP) } as *mut u8,
        }
    }
}

impl<const CAP: usize> Drop for Buffer<CAP> {
    fn drop(&mut self) {
        if self.ptr != null_mut() {
            unsafe {
                libc::free(self.ptr as *mut libc::c_void);
            }
        }
    }
}

impl<const CAP: usize> Deref for Buffer<CAP> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, CAP) }
    }
}

impl<const CAP: usize> DerefMut for Buffer<CAP> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, CAP) }
    }
}

unsafe impl<const CAP: usize> Send for Buffer<CAP> {}

/// Stores [Page]'s in a more compact compressed format, only storing the used
/// memory of a `Page`.
struct ZswapPage {
    /// Underlying memory buffer.
    ///
    /// Kept small (page size), so they can be cheaply defragmented by
    /// rebuilding the entire page.
    buf: Buffer<{ 4 << 10 }>,

    /// List of free memory ranges
    free_list: FreeList,
    //
    // TODO: page registry with access times
}

impl ZswapPage {
    /// Construct the page with a preallocated buffer
    fn new() -> Self {
        Self {
            buf: Buffer::new(),
            free_list: FreeList::new(4 << 10),
        }
    }
}

/// Page functionality protected by a mutex
struct PageInner {
    buffer: Buffer<{ 4 << 10 }>,
}

impl Drop for PageInner {
    fn drop(&mut self) {
        ALLOCATOR.lock().unwrap().release_page(self);
    }
}

/// 4 KB page for column, index and aggregate allocations
pub struct Page(RwLock<PageInner>);

// Swapping, compressing table, aggregate and index allocator
#[derive(Default)]
struct Allocator {
    /// Underlying 4 KB memory pages for swapping [Page] into
    zswap_pages: VecDeque<ZswapPage>,

    /// Unused allocated in-memory pages
    free_pages: Vec<Buffer<{ 4 << 10 }>>,
    //
    // TODO: each 100 ms (configurable) defragment up to 4 pages from the back
    // and move them to the front
    //
    // TODO: HashMap for finding pages by ID
    //
    // TODO: see, if we can somehow cheaply perform opportunistic
    // defragmentation on dump to disk
    //
    // TODO: each compressed page in a file dumped to disk should be its own LZ4
    // buffer, so that you can read them one by one, as needed
    // TODO: a page being read from disk should not block the allocator. We can
    // block the requesting thread instead.
    //
    // TODO: algo for determining, if a ZSWAPed page should be dumped to disk:
    // - dump everything older than a minute (configurable) + add t/o between
    //   dumps (configurable)
    // - if the amount of ZSWAP pages reaches a threshold (70%, configurable),
    //   dump until the threshold + add t/o between dumps (configurable)
}

// Allocator is only accessed from behind a mutex, so this is fine
unsafe impl Send for Allocator {}

impl Allocator {
    fn get_page(&mut self) -> Result<Arc<Page>, String> {
        todo!()
    }

    fn release_page(&mut self, p: &mut PageInner) {
        // Return buffer to allocator
        self.free_pages.push(Buffer { ptr: p.buffer.ptr });
        p.buffer.ptr = null_mut(); // Prevent double free

        todo!("unregister page")
    }
}

/// Acquire a 4 KB page for column, index and aggregate allocations
pub fn get_page() -> Result<Arc<Page>, String> {
    ALLOCATOR.lock().unwrap().get_page()
}
