extern crate alloc;

use core::mem::ManuallyDrop;
use core::slice;

use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::mm::get_kernel_end;
use crate::sync::spin::SpinLock;

pub(super) static PHYS_MEM_START: usize = 0x8000_0000;
pub(super) static PHYS_MEM_BYTES: usize = 128 * 1024 * 1024; // 128 MB

pub(super) static PAGE_SIZE_ORDER: usize = 12;
pub(super) static PAGE_SIZE_BYTES: usize = 1 << PAGE_SIZE_ORDER; // 4 KB

lazy_static! {
    /// Global allocator for physical memory pages. Pages before
    /// the end of kernel are treated as persistently allocated and
    /// will not be recycled.
    ///
    /// # Invariants
    /// * Access to pages that are not yet allocated or have been
    /// recycled should be forbidden, e.g., through virtual memory
    /// control.
    static ref PAGE_ALLOCATOR: SpinLock<PageAllocator> = SpinLock::new(PageAllocator {
        next_unused_ppn: compute_first_unused_ppn(),
        max_ppn: compute_max_ppn(),
        recycled: Vec::new(),
    });
}

fn compute_first_unused_ppn() -> PPN {
    let kernel_end = get_kernel_end();
    if kernel_end & (PAGE_SIZE_BYTES - 1) == 0 {
        PPN::from_addr(kernel_end)
    } else {
        PPN::from_addr(kernel_end + PAGE_SIZE_BYTES)
    }
}

/// Returns the exclusive upper bound of valid [PPN].
fn compute_max_ppn() -> PPN {
    assert!(
        PHYS_MEM_START & (PAGE_SIZE_BYTES - 1) == 0,
        "The algorithm assumes PYHS_MEM_START is page-aligned"
    );
    assert!(
        PHYS_MEM_BYTES & (PAGE_SIZE_BYTES - 1) == 0,
        "The algorithm assumes PYHS_MEM_BYTES is page-aligned"
    );

    PPN::from_addr(PHYS_MEM_START + PHYS_MEM_BYTES)
}

/// Allocates and returns a [Page]. If no page is available, returns
/// [None].
///
/// The allocated [Page] may contain old data.
fn alloc_page() -> Option<Page> {
    PAGE_ALLOCATOR.lock().alloc()
}

/// Allocates and returns a [Page]. If no page is available, returns
/// [None].
///
/// The allocated [Page] is zerod.
pub(super) fn alloc_zeroed_page() -> Option<Page> {
    let mut result = alloc_page()?;
    unsafe {
        // SAFETY: According to the invariants of [PAGE_ALLOCATOR],
        // no access to the newly allocated page is valid before
        // we hand it out. Therefore, we can safely overwrite the
        // content of the page.
        slice::from_raw_parts_mut(result.as_mut_ptr(), PAGE_SIZE_BYTES).fill(0);
    }
    Some(result)
}

fn dealloc_pages(pages: Vec<Page>) {
    let mut allocator = PAGE_ALLOCATOR.lock();
    for page in pages {
        let page = ManuallyDrop::new(page);
        allocator.dealloc(&page);
    }
}

struct PageAllocator {
    next_unused_ppn: PPN,
    /// The exclusive upper bound of valid [PPN].
    max_ppn: PPN,
    recycled: Vec<PPN>,
}

impl PageAllocator {
    fn alloc(&mut self) -> Option<Page> {
        if let Some(ppn) = self.recycled.pop() {
            return Some(Page { ppn });
        }

        if self.next_unused_ppn >= self.max_ppn {
            return None;
        }

        let result = Page {
            ppn: self.next_unused_ppn,
        };
        self.next_unused_ppn = PPN(self.next_unused_ppn.0 + 1);
        return Some(result);
    }

    fn dealloc(&mut self, page: &Page) {
        self.recycled.push(page.get_ppn());
    }
}

/// Abstraction of physical memory pages. It implements [Drop] to
/// automate page deallocation.
///
/// # Invariants
/// * Instance of [Page] should only be created by [PAGE_ALLOCATOR].
#[derive(Debug)]
pub(super) struct Page {
    ppn: PPN,
}

impl Page {
    pub(super) fn get_ppn(&self) -> PPN {
        self.ppn
    }

    // Returns a raw pointer to the start of the page.
    fn as_ptr(&self) -> *const u8 {
        (self.get_ppn().0 << PAGE_SIZE_ORDER) as *const u8
    }

    // Returns a raw pointer to the start of the page.
    fn as_mut_ptr(&mut self) -> *mut u8 {
        (self.get_ppn().0 << PAGE_SIZE_ORDER) as *mut u8
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        PAGE_ALLOCATOR.lock().dealloc(&self);
    }
}

/// Physical Page Number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PPN(pub(super) usize);

impl PPN {
    fn from_addr(addr: usize) -> Self {
        PPN(addr >> PAGE_SIZE_ORDER)
    }
}

#[allow(dead_code)]
pub(super) fn test_page_allocator() {
    assert_eq!(
        PAGE_ALLOCATOR.lock().recycled.len(),
        0,
        "This test assumes no recycled pages at the start"
    );

    // Allocate and deallocate a single page
    let old_next_unused_ppn = PAGE_ALLOCATOR.lock().next_unused_ppn;
    let page = alloc_page().expect("Failed to allocate page");
    assert_eq!(page.get_ppn(), old_next_unused_ppn);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled.len(), 0);
    drop(page);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled.len(), 1);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled[0], old_next_unused_ppn);

    // Allocate and deallocate a single page when there are recycled pages
    let old_next_unused_ppn = PAGE_ALLOCATOR.lock().next_unused_ppn;
    let page = alloc_page().expect("Failed to allocate page");
    assert_ne!(page.get_ppn(), old_next_unused_ppn);
    assert_eq!(PAGE_ALLOCATOR.lock().next_unused_ppn, old_next_unused_ppn);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled.len(), 0);
    drop(page);

    // Allocate and deallocate a single zerod page
    let mut page = alloc_zeroed_page().expect("Failed to allocate page");
    let ptr = page.as_mut_ptr();
    unsafe {
        for i in 0..PAGE_SIZE_BYTES {
            ptr.add(i).write(2);
        }
    };
    drop(page);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled.len(), 1);
    let page = alloc_zeroed_page().expect("Failed to allocate page");
    let ptr = page.as_ptr();
    unsafe {
        for i in 0..PAGE_SIZE_BYTES {
            assert_eq!(ptr.add(i).read(), 0);
        }
    }

    // Allocate and deallocate multiple pages
    let mut pages = Vec::new();
    for _ in 0..10 {
        pages.push(alloc_page().expect("Failed to allocate page"));
    }
    assert_eq!(PAGE_ALLOCATOR.lock().recycled.len(), 0);
    let page = pages.pop().unwrap();
    dealloc_pages(pages);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled.len(), 9);
    drop(page);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled.len(), 10);
}
