// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/heap.rs
//! Dynamic memory manager
use core::ptr::Unique;

// TODO: Rewrite this to correctly use the size information avaliable

use self::heapdef::HeapDef;

mod heapdef;

// --------------------------------------------------------
// Types
#[derive(Copy,Clone)]
pub enum HeapId
{
	Local,	// Inaccessible outside of process
	Global,	// Global allocations
}

#[derive(Debug)]
pub enum Error
{
	Corrupted,
	OutOfReservation,
	OutOfMemory,
}

//pub struct AnyAlloc
//{
//	ptr: Unique<()>,
//}
//pub struct TypedAlloc<T>
//{
//	ptr: Unique<T>,
//}
pub struct ArrayAlloc<T>
{
	ptr: Unique<T>,
	count: usize,
}
impl<T> !::lib::POD for ArrayAlloc<T> {}

// --------------------------------------------------------

pub const ZERO_ALLOC: *mut () = 1 as *mut _;

//static S_LOCAL_HEAP: ::sync::Mutex<HeapDef> = mutex_init!(HeapDef{head:None});
static S_GLOBAL_HEAP: ::sync::Mutex<HeapDef> = ::sync::Mutex::new(HeapDef::new());

// --------------------------------------------------------
// Code
pub fn init()
{
}

// Used by Box<T>
#[lang="exchange_malloc"]
#[inline]
unsafe fn exchange_malloc(size: usize, align: usize) -> *mut u8
{
	match allocate(HeapId::Global, size, align)
	{
	Some(x) => x as *mut u8,
	None => panic!("exchange_malloc({}, {}) out of memory", size, align),
	}
}
#[lang="exchange_free"]
#[inline]
unsafe fn exchange_free(ptr: *mut u8, size: usize, align: usize)
{
	S_GLOBAL_HEAP.lock().deallocate(ptr as *mut (), size, align)
}
#[lang = "box_free"]
#[inline]
unsafe fn box_free<T>(ptr: *mut T) {
	let size = ::core::mem::size_of::<T>();
	if size != 0 {
		S_GLOBAL_HEAP.lock().deallocate(ptr as *mut (), size, ::core::mem::align_of::<T>());
	}
}

// Used by libgcc and ACPICA
#[no_mangle] pub unsafe extern "C" fn malloc(size: usize) -> *mut () {
	allocate(HeapId::Global, size, 16).unwrap()
} 
#[no_mangle] pub unsafe extern "C" fn free(ptr: *mut ()) {
	if !ptr.is_null() {
		deallocate(ptr, 0, 16)
	}
} 

// Used by kernel internals
pub unsafe fn alloc<T>(value: T) -> *mut T
{
	let ret = match allocate(HeapId::Global, ::core::mem::size_of::<T>(), ::core::mem::align_of::<T>())
		{
		Some(v) => v as *mut T,
		None => panic!("Out of memory")
		};
	::core::ptr::write(ret, value);
	ret
}
pub unsafe fn alloc_raw(size: usize, align: usize) -> *mut () {
	match allocate(HeapId::Global, size, align)
	{
	Some(v) => v,
	None => panic!("Out of memory")
	}
}
pub unsafe fn dealloc<T>(value: *mut T)
{
	deallocate(value as *mut (), ::core::mem::size_of::<T>(), ::core::mem::align_of::<T>());
}
pub unsafe fn dealloc_raw(ptr: *mut (), size: usize, align: usize) {
	deallocate(ptr, size, align);
}

impl<T> ArrayAlloc<T>
{
	///// Create a new empty array allocation (const)
	//pub const fn empty() -> ArrayAlloc<T> {
	//	ArrayAlloc {
	//		// SAFE: Non-zero value
	//		ptr: unsafe { Unique::new(ZERO_ALLOC as *mut T) },
	//		count: 0
	//		}
	//}
	
	/// Create a new array allocation with `count` items
	pub fn new(count: usize) -> ArrayAlloc<T>
	{
		// SAFE: Correctly constructs 'Unique' instances
		unsafe {
			if ::core::mem::size_of::<T>() == 0 {
				ArrayAlloc { ptr: Unique::new(ZERO_ALLOC as *mut T), count: !0 }
			}
			else if count == 0 {
				ArrayAlloc { ptr: Unique::new(ZERO_ALLOC as *mut T), count: 0 }
			}
			else
			{
				let ptr = match allocate(HeapId::Global, ::core::mem::size_of::<T>() * count, ::core::mem::align_of::<T>())
					{
					Some(v) => v as *mut T,
					None => panic!("Out of memory when allocating array of {} elements", count)
					};
				assert!(!ptr.is_null());
				ArrayAlloc { ptr: Unique::new(ptr), count: count }
			}
		}
	}
	pub unsafe fn from_raw(ptr: *mut T, count: usize) -> ArrayAlloc<T> {
		ArrayAlloc { ptr: Unique::new(ptr), count: count }
	}
	pub fn into_raw(self) -> *mut [T] {
		let ptr = *self.ptr;
		let count = self.count;
		::core::mem::forget(self);
		// SAFE: Takes ownership
		unsafe {
			::core::slice::from_raw_parts_mut(ptr, count)
		}
	}
	
	pub fn count(&self) -> usize { self.count }
	
	pub fn get_base(&self) -> *const T { *self.ptr }
	pub fn get_base_mut(&mut self) -> *mut T { *self.ptr }
	
	#[tag_safe(irq)]
	pub fn get_ptr_mut(&mut self, idx: usize) -> *mut T {
		// SAFE: Index asserted to be valid, have &mut
		unsafe {
			assert!(idx < self.count, "ArrayAlloc<{}>::get_mut({}) OOB {}", type_name!(T), idx, self.count);
			self.ptr.offset(idx as isize)
		}
	}
	#[tag_safe(irq)]
	pub fn get_ptr(&self, idx: usize) -> *const T {
		// SAFE: Index asserted to be valid
		unsafe {
			assert!(idx < self.count, "ArrayAlloc<{}>::get_ptr({}) OOB {}", type_name!(T), idx, self.count);
			self.ptr.offset(idx as isize)
		}
	}

	/// Attempt to expand this array without reallocating	
	pub fn expand(&mut self, new_count: usize) -> bool
	{
		if new_count > self.count
		{
			let newsize = ::core::mem::size_of::<T>() * new_count;
			// SAFE: Pointer is valid
			if unsafe { expand( *self.ptr as *mut(), newsize ) }
			{
				self.count = new_count;
				true
			}
			else
			{
				false
			}
		}
		else
		{
			log_warning!("ArrayAlloc<{}>::expand: Called with <= count", type_name!(T));
			true
		}
	}
	
	pub fn shrink(&mut self, new_count: usize)
	{
		if new_count == self.count
		{
			// Nothing to do
		}
		else if new_count > self.count
		{
			log_warning!("ArrayAlloc::<{}>::shrink - Called with > count", type_name!(T));
		}
		else
		{
			let newsize = ::core::mem::size_of::<T>() * new_count;
			// SAFE: Pointer is valid, and raw pointer is being manipulated (lifetimes up to the caller)
			unsafe { shrink(*self.ptr as *mut (), newsize) };
			self.count = new_count;
		}
	}
}
impl_fmt!{
	<T> Debug(self,f) for ArrayAlloc<T> {
		write!(f, "ArrayAlloc {{ {:p} + {} }}", *self.ptr, self.count)
	}
}
impl<T> ::core::ops::Drop for ArrayAlloc<T>
{
	fn drop(&mut self)
	{
		if self.count > 0 {
			// SAFE: Pointer is valid
			unsafe { deallocate(*self.ptr as *mut (), ::core::mem::size_of::<T>() * self.count, ::core::mem::align_of::<T>()) };
		}
	}
}

// Main entrypoints
/// Allocate memory from the specified heap
unsafe fn allocate(heap: HeapId, size: usize, align: usize) -> Option<*mut ()>
{
	match heap
	{
	HeapId::Global => match S_GLOBAL_HEAP.lock().allocate(size, align)
		{
		Ok(v) => Some(v),
		Err(e) => {
			log_error!("Unable to allocate: {:?}", e);
			None
			},
		},
	_ => panic!("TODO: Non-global heaps"),
	}
}

/// Attempt to expand in-place
unsafe fn expand(pointer: *mut (), newsize: usize) -> bool
{
	S_GLOBAL_HEAP.lock().expand_alloc(pointer, newsize)
}
unsafe fn shrink(pointer: *mut (), newsize: usize)
{
	S_GLOBAL_HEAP.lock().shrink_alloc(pointer, newsize)
}

unsafe fn deallocate(pointer: *mut (), size: usize, align: usize)
{
	S_GLOBAL_HEAP.lock().deallocate(pointer as *mut (), size, align);
}


// vim: ft=rust
