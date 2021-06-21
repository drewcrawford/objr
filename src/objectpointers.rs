/*! object pointer types

A word on type design here.
First we have the objc dimension.  That is, NSObject, NSString, etc.  These
types are generally implemented as bare (zero-size) types.

Then we have the "cells".  These are smart, typed pointers implemented as newtypes.

For unsafe types, we have:

0.  RawCell - equivalent to objc `id`.  May be `nil`
1.  UnwrappedCell - thought *not* to be nil.

For safe types:

2.  AutoreleasedCell - part of an autorelease pool
2.  StrongCell - Compiler emits retain/release calls.
3.  RefCell - Borrowed pointer, retain/release elided.  Usually the one you want.

See documentation for particular cells.
*/

use core::ffi::{c_void};
use crate::bindings::{ActiveAutoreleasePool,GuaranteedMarker,ObjcInstance};
use std::marker::PhantomData;
use crate::objcinstance::ObjcInstanceBehavior;

const DEBUG_MEMORY: bool = false;


#[link(name="objc", kind="dylib")]
extern "C" {
    fn objc_retain(ptr: *mut  c_void) -> *const c_void;
    fn objc_release(ptr: *mut c_void);
    fn objc_autorelease(ptr: *mut c_void);
}

/**An typed, unsafe objc pointer, thought not to be `nil`.

because either we checked it, or because of `NS_ASSUME_NONNULL`, `NSObjectProtocol` in Swift, etc.

This is broadly similar to [GuaranteedMarker], the difference is that we can deref to objc_type here,
whereas for [GuaranteedMarker] we cannot.  This will also execute [std::ops::Drop] for T.
*/
#[derive(Debug)]
pub struct UnwrappedCell<T> {
    objc_type: T
}

impl<T: ObjcInstance> UnwrappedCell<T> {
    ///Unsafe because we don't check if the memory is valid
    pub unsafe fn new(marker: GuaranteedMarker<T>) -> Self {
        UnwrappedCell { objc_type: T::new(marker) }
    }

    ///Unsafe clone operation.
    ///
    /// # Safety
    /// This is unsafe because
    /// * We consider most lowlevel pointer ops unsafe generally (see [objc_instance!()#Safety])
    /// * Use of this method may allow multiple mutable references, which is plausibly unsafe although it's debatable.  (see [objc_instance!()#Mutability]).
    unsafe fn unsafe_clone(&self) -> Self {
        UnwrappedCell { objc_type: self.objc_type.unsafe_clone()}
    }
    ///Converts to [AutoreleasedCell] assuming [Self] was autoreleased.
    pub unsafe fn assuming_autoreleased<'a>(self, autorelease_pool: &'a ActiveAutoreleasePool) -> AutoreleasedCell<'a, T> {
        AutoreleasedCell::assuming_autoreleased(self,autorelease_pool)
    }
    ///Raw objc retain call, returns void
    ///
    /// Unsafe because obviously this is unsafe, come on.  We don't even allow this name in modern swift
    unsafe fn _retain(&self) {
        objc_retain(self.objc_type.unsafe_clone().marker().into_raw_ptr());
    }
    ///Raw objc autorelease call, returns void
    unsafe fn _autorelease(&self) {
        objc_autorelease(self.objc_type.unsafe_clone().marker().into_raw_ptr())
    }
    ///Converts to [StrongCell] by calling [`_retain(&self)`]
    ///
    ///Unsafe since
    /// * may not be a valid object.
    /// * We consider most lowlevel pointer ops unsafe generally (see [objc_instance!()#Safety])
    /// This is written to consume the underlying UnwrappedCell which is safe*er*.  The alternative
    /// may allow multiple mutable references, which is plausibly unsafe although it's debatable.  (see [objc_instance!()#Mutability]).
    pub unsafe fn retaining(self) -> StrongCell<T> {
        StrongCell::retaining(self.unsafe_clone())
    }
    ///Converts to [StrongCell] by assuming [Self] is retained.
    ///
    /// This is usually the case for some objc methods with names like `new`, `copy`, etc.
    /// # Safety
    /// Unsafe because
    /// * UnwrappedCell generally does not check the object is still valid
    /// * object may not actually be +1 retained
    /// This is written to consume the underlying UnwrappedCell which is safe*er*.  The alternative
    /// would be allowing multiple mutable references, which is plausibly unsafe although it's debatable.  (see [objc_instance!()#Mutability]).
    pub unsafe fn assuming_retained(self) -> StrongCell<T> {
        StrongCell::assuming_retained(self.marker().unsafe_clone() )
    }

    ///Moves out of self into a `GuaranteedMarker`.
    ///
    /// Compare with calling `instance.marker()`
    ///
    /// # Design
    ///
    /// This is implemented on `UnwrappedCell` to force you to make your
    /// memory management choices before here.  We can't implement it on the wrapper
    /// type since you're unlikely to have an owned reference (`StrongCell` and friends own that).
    /// Similarly, we don't want to implement on the higherlevel pointers since when they
    /// drop they will do some releasing action which is almost certainly incompatible with
    /// using `GuaranteedMarker`.
    ///
    /// # Safety
    ///
    /// Marking this as safe since it merely transforms an existing pointer type
    pub fn into_marker(self) -> GuaranteedMarker<T> {
        //ok to move out here since we have owned ref of self
        //There is no moving `marker()` variant implemented so we use `unsafe_clone` instead
        unsafe{ self.objc_type.marker().unsafe_clone() }
    }
}


/*
Safe conversions to unsafe types
 */
impl<'a, T> From<AutoreleasedCell<'a, T>> for UnwrappedCell<T> {
    fn from(cell: AutoreleasedCell<'a, T>) ->  Self {
        UnwrappedCell { objc_type: cell.objc_type }
    }
}

//Converting from StrongCell is deliberately ommitted.  It's kinda unclear
//what this means (does it mean std::mem::forget(strong) ?)

impl<T: std::fmt::Display> std::fmt::Display for UnwrappedCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.objc_type.fmt(f)
    }
}

impl<T: ObjcInstance> std::ops::Deref for UnwrappedCell<T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T {
        &self.objc_type
    }
}

impl<T: ObjcInstance> std::ops::DerefMut for UnwrappedCell<T> {
    #[inline] fn deref_mut(&mut self) -> &mut T {
        &mut self.objc_type
    }
}



///A pointer type that is safe and known to be valid.
/// Implementors: StrongCell, AutoreleasedCell
///
/// This trait is sealed and cannot be implemented outside the crate
pub trait SafePointer<MarkerType>: super::private::Sealed {
    ///Cast the type into an `UnwrappedCell`
    ///
    /// # Safety
    /// This is unsafe because anything you would do with the `UnwrappedCell` is inherently unsafe.
    /// See the comment of pointer safety in [objc_instance!] for details on this view.
    ///
    /// Callers need to ensure that the reference to the underlying type
    /// lives as long as the returned type.
    ///
    unsafe fn into_unwrapped_cell(self) -> UnwrappedCell<MarkerType>;
}

/**
An objc object that is part of an autorelease pool

The pool is used to lexically scope the lifetime of the pointer.
*/
#[derive(Debug)]
pub struct AutoreleasedCell<'a, T> {
    objc_type: T,
    ///for lifetime
    marker: PhantomData<&'a T>
}

impl<'a, T: ObjcInstance> AutoreleasedCell<'a, T> {
    ///Converts to [Self] by assuming the [UnwrappedCell] is already autoreleased.
    ///
    /// This is the case for many objc methods, depending on convention.
    pub unsafe fn assuming_autoreleased(cell: UnwrappedCell<T>, _pool: &'a ActiveAutoreleasePool) -> Self {
        AutoreleasedCell {
            objc_type: cell.objc_type,
            marker: PhantomData::default()
        }
    }
    ///Converts to [Self] by autoreleasing the [UnwrappedCell].
    ///
    /// Unsafe due to the fact that [UnwrappedCell] may not be valid.
    unsafe fn _autoreleasing(cell: UnwrappedCell<T>, pool: &'a ActiveAutoreleasePool) -> Self {
        cell._autorelease();
        Self::assuming_autoreleased(cell, pool)
    }

    ///Converts to [Self] by autoreleasing the [SafePointer<T>].
    pub fn autoreleasing<SafeCell: SafePointer<T>>(cell: SafeCell, pool: &'a ActiveAutoreleasePool) -> Self {
        unsafe {
            Self::_autoreleasing(cell.into_unwrapped_cell(),pool)
        }
    }
}
impl<'a, T> super::private::Sealed for AutoreleasedCell<'a, T> { }
//is a safe type
impl<'a, T: ObjcInstance> SafePointer<T> for AutoreleasedCell<'a, T> {
    unsafe fn into_unwrapped_cell(self) -> UnwrappedCell<T> {
        UnwrappedCell { objc_type: self.objc_type}
    }
}

impl<'a, T: ObjcInstance> std::ops::Deref for AutoreleasedCell<'a, T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T {
        &self.objc_type
    }
}

impl<'a, T: ObjcInstance>  std::ops::DerefMut for  AutoreleasedCell<'a, T> {
    #[inline] fn deref_mut(&mut self) -> &mut T {
        &mut self.objc_type
    }
}

impl<'a, T: std::fmt::Display> std::fmt::Display for AutoreleasedCell<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.objc_type.fmt(f)
    }
}

/**
A strong pointer to an objc object.

When this type is created, we will `retain` (unless we assume +1 due to objc convention.)
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
pub struct StrongCell<T: ObjcInstance> {
    objc_type: T
}
impl<T: ObjcInstance> StrongCell<T> {
    ///Unsafe since we don't know if the object is still valid.
    pub unsafe fn retaining<IntoUnwrapped: Into<UnwrappedCell<T>>>(unwrapped_cell: IntoUnwrapped) -> Self {
        let c = unwrapped_cell.into();
        c._retain();
        //safe because `c` is owned here
        Self::assuming_retained(c.marker().unsafe_clone())
    }
    ///Converts to [Self] by assuming [UnwrappedCell] is already retained.
    ///
    /// This is usually the case for some objc methods with names like `new`, `copy`, `init`, etc.
    pub unsafe fn assuming_retained(guaranteed_marker: GuaranteedMarker<T>) -> Self {
        StrongCell { objc_type: T::new(guaranteed_marker) }
    }
    ///Converts to [AutoreleasedCell] by calling `autorelease` on `self`.
    ///
    ///Safe, but needs to be a moving function, because the StrongCell will not be valid once we
    /// decrement its reference counter.
    pub fn autoreleasing(self, pool: &ActiveAutoreleasePool) -> AutoreleasedCell<T> {
        AutoreleasedCell::autoreleasing(self, pool)
    }

    ///Converts to `UnwrappedCell`.
    ///
    /// # Safety
    ///
    /// This function LEAKS.  The underlying memory will not be decremented by Rust code.
    ///
    /// This pattern is useful when implementing a +1 return convention from Rust.
    pub fn leak(self) -> UnwrappedCell<T> {
        let ptr = unsafe{ self.objc_type.marker().unsafe_clone() };
        let unmanaged = unsafe{ UnwrappedCell::new(ptr) };
        std::mem::forget(self);
        unmanaged
    }
}


//is safe pointer
impl<T: ObjcInstance> super::private::Sealed for StrongCell<T> { }
impl<T: ObjcInstance> SafePointer<T> for StrongCell<T> {
    unsafe fn into_unwrapped_cell(self) -> UnwrappedCell<T> {
        UnwrappedCell { objc_type: self.objc_type.unsafe_clone()}
    }
}
impl<T: ObjcInstance> Drop for StrongCell<T> {
    fn drop(&mut self) {
        unsafe {
            if DEBUG_MEMORY {
                println!("Drop {} {:?}",std::any::type_name::<T>(), self.marker());
            }
            objc_release(self.objc_type.unsafe_clone().marker().into_raw_ptr());
        }
    }
}
impl<T: ObjcInstance> std::ops::Deref for StrongCell<T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T {
        &self.objc_type
    }
}
impl<T: ObjcInstance>  std::ops::DerefMut for  StrongCell<T> {
    #[inline] fn deref_mut(&mut self) -> &mut T {
        &mut self.objc_type
    }
}

impl<T: std::fmt::Display + ObjcInstance> std::fmt::Display for StrongCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.objc_type.fmt(f)
    }
}








