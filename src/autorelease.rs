//! Autorelease pools and similar

use core::ffi::{c_void};
use core::marker::PhantomData;

extern "C" {
    pub fn objc_autoreleasePoolPush() -> *const c_void;
    pub fn objc_autoreleasePoolPop(ptr: *const c_void);
}

///Marker type that indicates you have an active autorelease pool.
///
/// You will not be given an owned copy of this type, only a borrowed copy.
/// The type is created when the pool is pushed and destroyed when the pool is popped.
pub struct ActiveAutoreleasePool {
    ///don't allow anyone else to construct this
    /// !Send !Sync
    _marker: PhantomData<*const ()>
}

impl ActiveAutoreleasePool {
    ///This function makes the [ActiveAutoreleasePool] marker type guaranteeing we have an autoreleasepool
    /// active on the thread.
    ///
    /// This is generally unsafe, but if you know an autoreleasepool is active, and can't
    /// get to it for some reason (for example, implementing a fixed trait without the right arguments)
    /// then you can construct the marker type here.
    pub unsafe fn assuming_autoreleasepool() -> ActiveAutoreleasePool {
        ActiveAutoreleasePool {_marker: PhantomData::default() }
    }
}

///An internal guard type.
///
/// Implements drop and pops the pool.
struct AutoreleaseGuard {
    ptr: *const c_void,
}

///Pops the pool
impl Drop for AutoreleaseGuard {
    fn drop(&mut self) {
        unsafe{ objc_autoreleasePoolPop(self.ptr) }
    }
}

impl AutoreleaseGuard {
    unsafe fn new() -> Self {
        AutoreleaseGuard {
            ptr: objc_autoreleasePoolPush()
        }
    }
}

///Spawn a new autoreleasepoool and perform the closure on it.
///
/// The autoreleasepool will be valid for the duration of the closure, after which it will be popped.
pub fn autoreleasepool<T, F: FnOnce(&ActiveAutoreleasePool) -> T>(f: F) -> T {
    let _guard = unsafe { AutoreleaseGuard::new() };
    let pool = ActiveAutoreleasePool {_marker: PhantomData::default() };
    f(&pool)
}
