mod free_list;
mod linked_list;
mod lru_map;

use std::{
    borrow::BorrowMut,
    collections::VecDeque,
    ops::{Deref, DerefMut},
    ptr::null_mut,
    sync::RwLock,
    time::Instant,
    usize,
};

use self::free_list::FreeList;

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

/// Stores `Page`s in a more compact compressed format, only storing the used
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
        with_allocator(|a| a.release_page(self));
    }
}

/// 4 KB page for column, index and aggregate allocations
pub struct Page(RwLock<PageInner>);

// Swapping, compressing table, aggregate and index allocator
#[derive(Default)]
struct Allocator {
    /// Underlying 4 KB memory pages for swapping `Page`s into
    //
    // TODO: each 100 ms (configurable) defragment up to 4 pages from the back
    // and move them to the front
    //
    // TODO: some sort of data structure for enabling both finding a page by ID
    // and ordering pages by fragmentation to pick the least fragmented one for
    // new allocations
    zswap_pages: VecDeque<ZswapPage>,

    // TODO: page registry via LRU map
    // TODO: allow bumping usage time w/o allocator lock via separate mutex
    //
    /// Unused pages not yet returned to the operating system.
    /// Stored together with their monotonous insertion time.
    //
    // TODO: keep a small pool of pages (4?) in reserve for allocator purposes
    free_pages: VecDeque<(Instant, Buffer<{ 4 << 10 }>)>,
}

impl Allocator {
    fn get_page(&mut self) -> Result<Page, String> {
        todo!()
    }

    fn release_page(&mut self, p: &mut PageInner) {
        // Return buffer to allocator
        self.free_pages
            .push_back((Instant::now(), Buffer { ptr: p.buffer.ptr }));
        p.buffer.ptr = null_mut(); // Prevent double free

        todo!("unregister page")
    }
}

/// Run function with global page allocator as an argument, acquiring exclusive
/// access to it
fn with_allocator<F, R>(f: F) -> R
where
    F: FnOnce(&mut Allocator) -> R,
{
    use std::sync::{Mutex, Once};

    static ONCE: Once = Once::new();
    static mut ALLOCATOR: Option<Mutex<Allocator>> = None;
    ONCE.call_once(|| unsafe {
        ALLOCATOR = Some(Default::default());
    });

    f(&mut *unsafe { ALLOCATOR.as_ref().unwrap() }
        .lock()
        .unwrap()
        .borrow_mut())
}

/// Acquire a 4 KB page for column, index and aggregate allocations
pub fn get_page() -> Result<Page, String> {
    with_allocator(|a| a.get_page())
}
