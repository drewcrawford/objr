//! Autorelease pools and similar

use core::ffi::{c_void};
use core::marker::PhantomData;
use std::ops::Deref;

extern "C" {
    pub fn objc_autoreleasePoolPush() -> *const c_void;
    pub fn objc_autoreleasePoolPop(ptr: *const c_void);
}

///Marker type that indicates you have an active autorelease pool.
///
/// This type is generally appropriate for passing around as an argument.  In practice, it is zero-sized,
/// so it should be the zero-cost abstraction.
///
/// Generally, you work with borrows of this type.  The lifetime of the borrow
/// is the lifetime that the autoreleasepool is statically guaranteed to be active.  This lets
/// you check autorelease behavior statically.
///
/// There are two ways to construct this type:
/// 1.  by dereferencing an [AutoreleasePool] (preferred)
///2.   [ActiveAutoreleasePool::assuming_autoreleasepool()].
pub struct ActiveAutoreleasePool {
    ///don't allow anyone else to construct this
    /// !Send !Sync
    _marker: PhantomData<*const ()>
}

impl ActiveAutoreleasePool {
    ///This function makes the [ActiveAutoreleasePool] marker type guaranteeing we have an autoreleasepool
    /// active on the thread.
    ///
    /// # Safety
    /// This is generally unsafe, but if you are certain an autoreleasepool is active on the thread,
    /// you can use this constructor to create your own marker tpe.
    pub unsafe fn assuming_autoreleasepool() -> ActiveAutoreleasePool {
        ActiveAutoreleasePool {_marker: PhantomData::default() }
    }
}
///Tracks an active autoreleasepool.
///
/// This is generally used at the "top level" to create a new pool, for a
/// type to use as an argument instead, see [ActiveAutoreleasePool].
///
/// This type can be dereferenced into [ActiveAutoreleasePool].
///
/// Pops the pool on drop.
pub struct AutoreleasePool {
    // !Send, !Sync
    ptr: *const c_void,
    pool: ActiveAutoreleasePool,
}

impl Deref for AutoreleasePool {
    type Target = ActiveAutoreleasePool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

///Pops the pool
impl Drop for AutoreleasePool {
    fn drop(&mut self) {
        unsafe{ objc_autoreleasePoolPop(self.ptr) }
    }
}

impl AutoreleasePool {
    ///Creates a new pool.  The pool will be dropped when this type is dropped.
    pub fn new() -> Self {
        unsafe{
            AutoreleasePool {
                ptr: objc_autoreleasePoolPush(),
                pool: ActiveAutoreleasePool::assuming_autoreleasepool()
            }
        }
    }
}
