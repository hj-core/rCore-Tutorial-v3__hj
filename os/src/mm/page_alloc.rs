extern crate alloc;

use core::mem::ManuallyDrop;

use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::mm::{
    MEM_SIZE_BYTES, MEM_START_PA, PAGE_SIZE_BYTES, PPN, get_pa_from_va, get_pa_mut_ptr, kernel_end,
};
use crate::sync::spin::SpinLock;

lazy_static! {
    /// Global allocator for physical memory pages. Pages before
    /// the end of the kernel are treated as persistently allocated
    /// and will not be recycled.
    ///
    /// # Invariants
    /// * Access to pages that are not yet allocated or have been
    /// recycled should be forbidden, e.g., through virtual memory
    /// control.
    static ref PAGE_ALLOCATOR: SpinLock<PageAllocator> = {
        let next_unused_ppn = compute_first_unused_ppn();
        let max_ppn = compute_max_ppn();
        let recycled_ppn = Vec::new();
        SpinLock::new(PageAllocator { next_unused_ppn, max_ppn, recycled_ppn })
    };
}

fn compute_first_unused_ppn() -> PPN {
    let kernel_end_pa = get_pa_from_va(kernel_end as usize);

    if kernel_end_pa & (PAGE_SIZE_BYTES - 1) == 0 {
        PPN::from_pa(kernel_end_pa)
    } else {
        PPN::from_pa(kernel_end_pa + PAGE_SIZE_BYTES)
    }
}

/// Returns the exclusive upper bound of valid [PPN].
fn compute_max_ppn() -> PPN {
    assert_eq!(
        MEM_START_PA & (PAGE_SIZE_BYTES - 1),
        0,
        "The algorithm assumes MEM_START_PA is page-aligned"
    );
    assert_eq!(
        MEM_SIZE_BYTES & (PAGE_SIZE_BYTES - 1),
        0,
        "The algorithm assumes MEM_SIZE_BYTES is page-aligned"
    );

    PPN::from_pa(MEM_START_PA + MEM_SIZE_BYTES)
}

/// Allocates and returns a [Page]. If no page is available,
/// returns [None].
///
/// The allocated [Page] may contain old data.
pub(super) fn alloc_page() -> Option<Page> {
    PAGE_ALLOCATOR.lock().alloc()
}

/// Allocates and returns a [Page]. If no page is available,
/// returns [None].
///
/// The allocated [Page] is zerod.
pub(super) fn alloc_zeroed_page() -> Option<Page> {
    let result = alloc_page()?;
    let pa = result.get_ppn().get_pa();
    let pa_mut_ptr = get_pa_mut_ptr(pa);
    for i in 0..PAGE_SIZE_BYTES {
        unsafe { pa_mut_ptr.add(i).write(0) };
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
    recycled_ppn: Vec<PPN>,
}

impl PageAllocator {
    fn alloc(&mut self) -> Option<Page> {
        if let Some(ppn) = self.recycled_ppn.pop() {
            return Some(Page { ppn });
        }

        if self.next_unused_ppn >= self.max_ppn {
            return None;
        }

        let result = Page {
            ppn: self.next_unused_ppn,
        };
        self.next_unused_ppn = PPN(self.next_unused_ppn.0 + 1);
        Some(result)
    }

    fn dealloc(&mut self, page: &Page) {
        self.recycled_ppn.push(page.get_ppn());
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
}

impl Drop for Page {
    fn drop(&mut self) {
        PAGE_ALLOCATOR.lock().dealloc(&self);
    }
}

#[allow(dead_code)]
pub(super) fn test_page_allocator() {
    assert_eq!(
        PAGE_ALLOCATOR.lock().recycled_ppn.len(),
        0,
        "This test assumes no recycled pages at the start"
    );

    // Allocate and deallocate a single page
    let old_next_unused_ppn = PAGE_ALLOCATOR.lock().next_unused_ppn;
    let page = alloc_page().expect("Failed to allocate page");
    assert_eq!(page.get_ppn(), old_next_unused_ppn);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled_ppn.len(), 0);
    drop(page);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled_ppn.len(), 1);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled_ppn[0], old_next_unused_ppn);

    // Allocate and deallocate a single page when there are recycled pages
    let old_next_unused_ppn = PAGE_ALLOCATOR.lock().next_unused_ppn;
    let page = alloc_page().expect("Failed to allocate page");
    assert_ne!(page.get_ppn(), old_next_unused_ppn);
    assert_eq!(PAGE_ALLOCATOR.lock().next_unused_ppn, old_next_unused_ppn);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled_ppn.len(), 0);
    drop(page);

    // Allocate and deallocate a single zerod page
    let page = alloc_zeroed_page().expect("Failed to allocate page");
    let ptr = get_pa_mut_ptr(page.get_ppn().get_pa());
    unsafe {
        for i in 0..PAGE_SIZE_BYTES {
            ptr.add(i).write(2);
        }
    };
    drop(page);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled_ppn.len(), 1);
    let page = alloc_zeroed_page().expect("Failed to allocate page");
    let ptr = get_pa_mut_ptr(page.get_ppn().get_pa());
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
    assert_eq!(PAGE_ALLOCATOR.lock().recycled_ppn.len(), 0);
    let page = pages.pop().unwrap();
    dealloc_pages(pages);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled_ppn.len(), 9);
    drop(page);
    assert_eq!(PAGE_ALLOCATOR.lock().recycled_ppn.len(), 10);
}
