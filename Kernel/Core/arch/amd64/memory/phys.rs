// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/memory/phys.rs
//! Physical address space managment
//!
//! Handles reference counting and allocation bitmaps
use arch::imp::memory::addresses::{PMEMREF_BASE,PMEMREF_END,PMEMBM_BASE,PMEMBM_END};
use sync::RwLock;
use core::sync::atomic::{Ordering};
use sync::AtomicU32;
use memory::page_array::{PageArray};

// 1. Reference counts are maintained as a region of address space containing the reference counts
// 2. Bitmap (maybe?) maintained 

/// Multiref count array
static S_REFCOUNT_ARRAY: RwLock<PageArray<AtomicU32>> = RwLock::new( PageArray::new(PMEMREF_BASE, PMEMREF_END) );
static S_USED_BITMAP: RwLock<PageArray<AtomicU32>> = RwLock::new( PageArray::new(PMEMBM_BASE, PMEMBM_END) );

/// Calls the provided closure with a borrow of the reference count for the specified frame
fn with_ref<U, F: FnOnce(&AtomicU32)->U>(frame_idx: u64, fcn: F) -> Option<U>
{
	S_REFCOUNT_ARRAY.read().get(frame_idx as usize).map(fcn)
}
/// Calls the provided closure with a reference to the specified frame's reference count (allocating if required)
fn with_ref_alloc<U, F: FnOnce(&AtomicU32)->U>(frame_idx: u64, fcn: F) -> U
{
	let mut lh = S_REFCOUNT_ARRAY.write();
	fcn( lh.get_alloc(frame_idx as usize) )
}
/// Calls the provided closure with a borrow of the reference count for the specified frame
fn with_bm<U, F: FnOnce(&AtomicU32)->U>(ofs: usize, fcn: F) -> Option<U>
{
	S_USED_BITMAP.read().get(ofs).map(fcn)
}
/// Calls the provided closure with a reference to the specified frame's reference count (allocating if required)
fn with_bm_alloc<U, F: FnOnce(&AtomicU32)->U>(ofs: usize, fcn: F) -> U
{
	let mut lh = S_USED_BITMAP.write();
	fcn( lh.get_alloc(ofs) )
}


pub fn ref_frame(frame_idx: u64) {
	with_ref_alloc( frame_idx, |r| r.fetch_add(1, Ordering::Acquire) );
}
pub fn deref_frame(frame_idx: u64) -> u32 {
	with_ref(frame_idx, |r|
		if r.load(Ordering::Relaxed) != 0 {
			r.fetch_sub(1, Ordering::Release)
		}
		else {
			0
		}
		).unwrap_or(0)
}
pub fn get_multiref_count(frame_idx: u64) -> u32 {
	with_ref( frame_idx, |r| r.load(Ordering::Relaxed) ).unwrap_or(0)
}

/// Returns true if the frame was marked as allocated
pub fn mark_free(frame_idx: u64) -> bool {
	let mask = 1 << ((frame_idx % 32) as usize);
	with_bm( (frame_idx / 32) as usize, |c| {
		let mut old = c.load(Ordering::Relaxed);
		if old & mask == 0
		{
			// Bit was clear, frame was already free?
			false
		}
		else {
			// Bit set, loop until a compare+swap succeeds
			loop
			{
				let new_old = c.compare_and_swap(old, old & !mask, Ordering::Relaxed);
				if old == new_old {
					break ;
				}
				old = new_old;
			}
			true
		}
		}).unwrap_or(false)
}
pub fn mark_used(frame_idx: u64) {
	let mask = 1 << ((frame_idx % 32) as usize);
	with_bm_alloc( (frame_idx / 32) as usize, |c| {
		// Should always succeed due to write lock in `with_bm_alloc`
		let old = c.load(Ordering::Relaxed);
		let new_old = c.compare_and_swap(old, old | mask, Ordering::Relaxed);
		assert_eq!(new_old, old);
		})
}



