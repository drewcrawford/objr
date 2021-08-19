/*! object pointer types

For safe types:

1.  AutoreleasedCell - part of an autorelease pool
2.  StrongCell - Compiler emits retain/release calls.

Mutable variants (not yet implemented todo):

1.  AutoreleasedMutableCell - like [AutoreleasedCell] but mutable
2.  StrongMutableCell - like [StrongCell] but mutable

See documentation for particular cells.
*/

use core::ffi::{c_void};
use crate::bindings::{ActiveAutoreleasePool,ObjcInstance};
use std::marker::PhantomData;
use crate::objcinstance::NonNullImmutable;

///Turning this on may help debug retain/release
const DEBUG_MEMORY: bool = false;


#[link(name="objc", kind="dylib")]
extern "C" {
    fn objc_retain(ptr: *const  c_void) -> *const c_void;
    fn objc_release(ptr: *const c_void);
    fn objc_autorelease(ptr: *const c_void);
}


/**
An objc object that is part of an autorelease pool

The pool is used to lexically scope the lifetime of the pointer.
*/
#[derive(Debug)]
pub struct AutoreleasedCell<'a, T> {
    ptr: NonNullImmutable<T>,
    ///for lifetime
    marker: PhantomData<&'a T>
}

impl<'a, T: ObjcInstance> AutoreleasedCell<'a, T> {

    ///Converts to [Self] by autoreleasing the [SafePointer<T>].
    pub fn autoreleasing(cell: &T, _pool: &'a ActiveAutoreleasePool) -> Self {
        unsafe {
            objc_autorelease(cell as *const _ as *const c_void)
        }
        Self{
            ptr: NonNullImmutable::from_reference(cell),
            marker: Default::default()
        }
    }
}
impl<'a, T: ObjcInstance> AutoreleasedCell<'a, T> {
    ///Converts to [Self] by assuming the [UnwrappedCell] is already autoreleased.
    ///
    /// This is the case for many objc methods, depending on convention.
    pub unsafe fn assuming_autoreleased(ptr: NonNullImmutable<T>, _pool: &'a ActiveAutoreleasePool) -> Self {
        AutoreleasedCell {
            ptr,
            marker: PhantomData::default()
        }
    }
}

impl<'a, T: ObjcInstance> std::ops::Deref for AutoreleasedCell<'a, T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T {
        unsafe{ &*self.ptr.as_ptr() }
    }
}


impl<'a, T: ObjcInstance> std::fmt::Display for AutoreleasedCell<'a, T> where *const T: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.ptr.as_ptr().fmt(f)
    }
}

/**
A strong pointer to an objc object.

This is often the type you want as the return
type when implementing an ObjC binding.

When this type is created, we will `retain` (unless using an unsafe [assuming_retained()] constructor)
When the obj is dropped, we will `release`.

In ObjC, the compiler tries to elide retain/release but it
may not be possible due to lack of global knowledge, in which
case it inserts `retain` as a precaution.

In Rust we have global knowledge of lifetimes so we can
elide more perfectly.  However this requires splitting up
objc `strong` into an explicit typesystem.

This type emits `retain`/`release` unconditionally.  Therefore
you can think of it like the "worst case" of objc `strong`, the
case where the compiler cannot elide anything.  You can also think of
it as a "lifetime eraser", that is we erase knowledge of the object lifetime,
so we assume we need to retain.

This is often used at the border of an objc binding.

For an elided 'best case' version, see `RefCell`.
*/
#[derive(Debug)]
pub struct StrongCell<T: ObjcInstance>(NonNullImmutable<T>);
impl<T: ObjcInstance> StrongCell<T> {
    pub fn retaining(cell: &T) -> Self {
        unsafe {
            objc_retain(cell as *const T as *const c_void);
            Self::assuming_retained(cell)
        }
    }

    ///Converts to [AutoreleasedCell] by calling `autorelease` on `self`.
    ///
    ///Safe, but needs to be a moving function, because the StrongCell will not be valid once we
    /// decrement its reference counter.
    pub fn autoreleasing<'a>(cell: &Self, pool: &'a ActiveAutoreleasePool) -> AutoreleasedCell<'a, T> {
        AutoreleasedCell::autoreleasing(cell, pool)
    }

}

impl<T: ObjcInstance> StrongCell<T> {
    ///Converts to [Self] by assuming the argument is already retained.
    ///
    /// This is usually the case for some objc methods with names like `new`, `copy`, `init`, etc.
    /// # Safety
    /// If this isn't actually retained, will UB
    pub unsafe fn assuming_retained(reference: &T) -> Self {
        StrongCell(NonNullImmutable::from_reference(reference))
    }
}

impl<T: ObjcInstance> Drop for StrongCell<T> {
    fn drop(&mut self) {
        unsafe {
            if DEBUG_MEMORY {
                println!("Drop {} {:p}",std::any::type_name::<T>(), self);
            }
            objc_release(self.0.as_ptr() as *const _ as *const c_void);
        }
    }
}
impl<T: ObjcInstance> std::ops::Deref for StrongCell<T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T {
        unsafe{ &*self.0.as_ptr()}
    }
}

impl<'a, T: ObjcInstance> std::fmt::Display for StrongCell<T> where T: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ptr = unsafe{ &*(self.0.as_ptr())};
        f.write_fmt(format_args!("{}",ptr))
    }
}








