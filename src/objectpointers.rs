/*! object pointer types

For safe types:

1.  AutoreleasedCell - part of an autorelease pool
2.  StrongCell - Compiler emits retain/release calls.

Mutable variants:

1.  AutoreleasedMutCell - like [AutoreleasedCell] but mutable
2.  StrongMutCell - like [StrongCell] but mutable

Lifetime variants:
1.  StrongLifetimeCell - like [StrongCell] but tracks some explicit lifetime.  Often used for objects that borrow Rust storage.


See documentation for particular cells.
 */

use core::ffi::{c_void};
use crate::bindings::{ActiveAutoreleasePool,ObjcInstance};
use std::marker::PhantomData;
use crate::objcinstance::NonNullImmutable;
use std::ptr::NonNull;
use std::fmt::{Debug};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use crate::objcinstance::ObjcInstanceBehavior;

extern "C" {
    fn objc_autoreleaseReturnValue(object: *const c_void) -> *const c_void;
}

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

    ///Converts to [Self] by autoreleasing the reference.
    pub fn autoreleasing(cell: &T, _pool: &'a ActiveAutoreleasePool) -> Self {
        unsafe {
            objc_autorelease(cell as *const _ as *const c_void)
        }
        Self{
            ptr: NonNullImmutable::from_reference(cell),
            marker: Default::default()
        }
    }
    ///Converts to [Self] by assuming the pointer is already autoreleased.
    ///
    /// This is the case for many objc methods, depending on convention.
    pub unsafe fn assume_autoreleased(ptr: &T, _pool: &'a ActiveAutoreleasePool) -> Self {
        if DEBUG_MEMORY {
            println!("assume_autoreleased {} {:p}",std::any::type_name::<T>(), ptr);
        }
        AutoreleasedCell {
            ptr: NonNullImmutable::from_reference(ptr),
            marker: PhantomData::default()
        }
    }

    ///Converts to a mutable version.
    ///
    /// # Safety
    /// You are responsible to check:
    /// * There are no other references to the type, mutable or otherwise
    /// * The type is in fact "mutable", whatever that means.  Specifically, to whatever extent `&mut` functions are forbidden
    ///   generally, you must ensure it is appropriate to call them here.
    pub unsafe fn assume_mut(self) -> AutoreleasedMutCell<'a, T> {
        let r =
            AutoreleasedMutCell {
                ptr: NonNull::new_unchecked(self.ptr.as_ptr() as *mut T),
                marker: Default::default()
            };
        std::mem::forget(self);
        r
    }
}
impl<'a, T: ObjcInstance> std::ops::Deref for AutoreleasedCell<'a, T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T {
        unsafe{ &*self.ptr.as_ptr() }
    }
}


impl<'a, T: ObjcInstance> std::fmt::Display for AutoreleasedCell<'a, T> where T: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ptr = unsafe{ &*self.ptr.as_ptr() };
        std::fmt::Display::fmt(ptr, f)
    }
}
impl<'a, T: PartialEq + ObjcInstance> PartialEq for AutoreleasedCell<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        let a: &T = self;
        let b: &T = other;
        a == b
    }
}

impl<'a, T: Eq + ObjcInstance> Eq for AutoreleasedCell<'a, T>  {}

impl<'a, T: Hash + ObjcInstance> Hash for AutoreleasedCell<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let a: &T = self;
        a.hash(state);
    }
}

/**
An objc object that is part of an autorelease pool

The pool is used to lexically scope the lifetime of the pointer.
 */
#[derive(Debug)]
pub struct AutoreleasedMutCell<'a, T> {
    ptr: NonNull<T>,
    ///for lifetime
    marker: PhantomData<&'a T>
}

impl<'a, T: ObjcInstance> AutoreleasedMutCell<'a, T> {

    ///Converts to [Self] by autoreleasing the reference.
    pub fn autoreleasing(cell: &mut T, _pool: &'a ActiveAutoreleasePool) -> Self {
        unsafe {
            objc_autorelease(cell as *const _ as *const c_void)
        }
        Self{
            ptr: unsafe{ NonNull::new_unchecked(cell) },
            marker: Default::default()
        }
    }
    ///Converts to [Self] by assuming the pointer is already autoreleased.
    ///
    /// This is the case for many objc methods, depending on convention.
    pub unsafe fn assume_autoreleased(ptr: &mut T, _pool: &'a ActiveAutoreleasePool) -> Self {
        if DEBUG_MEMORY {
            println!("assume_autoreleased {} {:p}",std::any::type_name::<T>(), ptr);
        }
        Self {
            ptr: NonNull::new_unchecked(ptr),
            marker: PhantomData::default()
        }
    }
}

impl<'a, T: ObjcInstance> std::ops::Deref for AutoreleasedMutCell<'a, T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T {
        unsafe{ &*self.ptr.as_ptr() }
    }
}
impl<'a, T: ObjcInstance> std::ops::DerefMut for AutoreleasedMutCell<'a, T> {
    #[inline] fn deref_mut(&mut self) -> &mut T {
        unsafe{ &mut *self.ptr.as_mut() }
    }
}


impl<'a, T: ObjcInstance> std::fmt::Display for AutoreleasedMutCell<'a, T> where T: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ptr = unsafe{ &*self.ptr.as_ptr() };
        f.write_fmt(format_args!("{}",ptr))
    }
}

impl<'a, T: PartialEq + ObjcInstance> PartialEq for AutoreleasedMutCell<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        let a: &T = self;
        let b: &T = other;
        a == b
    }
}
impl<'a, T: Eq + ObjcInstance> Eq for AutoreleasedMutCell<'a, T> {}
impl<'a, T: Hash + ObjcInstance> Hash for AutoreleasedMutCell<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let a: &T = self;
        a.hash(state);
    }
}

/**
A strong pointer to an objc object.

This is often the type you want as the return
type when implementing an ObjC binding.

When this type is created, we will `retain` (unless using an unsafe [StrongCell::assume_retained()] constructor)
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
            if DEBUG_MEMORY {
                println!("retain {} {:p}",std::any::type_name::<T>(), cell);
            }
            objc_retain(cell as *const T as *const c_void);
            Self::assume_retained(cell)
        }
    }

    ///Converts to [AutoreleasedCell] by calling `autorelease` on `self`.
    ///
    ///Safe, but needs to be a moving function, because the StrongCell will not be valid once we
    /// decrement its reference counter.
    pub fn autoreleasing<'a>(cell: &Self, pool: &'a ActiveAutoreleasePool) -> AutoreleasedCell<'a, T> {
        AutoreleasedCell::autoreleasing(cell, pool)
    }
    ///Converts to [Self] by assuming the argument is already retained.
    ///
    /// This is usually the case for some objc methods with names like `new`, `copy`, `init`, etc.
    /// # Safety
    /// You are responsible to check:
    /// * That the type is retained
    /// * That the type is 'static, that is, it has no references to external (Rust) memory.
    ///   If this is not the case, see [StrongLifetimeCell].
    pub unsafe fn assume_retained(reference: &T) -> Self {
        if DEBUG_MEMORY {
            println!("assume_retained {} {:p}",std::any::type_name::<T>(), reference);
        }
        StrongCell(NonNullImmutable::from_reference(reference))
    }

    ///Converts to a mutable version.
    ///
    /// # Safety
    /// You are responsible to check:
    /// * There are no other references to the type, mutable or otherwise
    /// * The type is in fact "mutable", whatever that means.  Specifically, to whatever extent `&mut` functions are forbidden
    ///   generally, you must ensure it is appropriate to call them here.
    pub unsafe fn assume_mut(self) -> StrongMutCell<T> {
        let r = StrongMutCell(
            NonNull::new_unchecked(self.0.as_ptr() as *mut T),
        );
        std::mem::forget(self);
        r
    }
    ///Attempts to use the "trampoline" trick to return an autoreleased value to objc.
    ///
    /// This is largely used when implementing a subclass.
    ///
    ///You must return the return value of this function, to your caller to get optimized results.
    /// Results are not guaranteed to be optimized, in part because inline assembly is not stabilized.
    #[inline(always)] pub fn return_autoreleased(self) -> *const T {
        let ptr = self.0.as_ptr();
        std::mem::forget(self); //LEAK
        unsafe{ objc_autoreleaseReturnValue(ptr as *const c_void) as *const T }
    }

    ///Reinterprets this cell as a cell of another type.
    ///
    /// # Performance
    /// This is a 0-cost abstraction.  The retain/release calls of converting to the new cell type are elided.
    /// # Safety
    /// You must comply with all the safety guarantees of [ObjcInstanceBehavior::cast].
    #[inline] pub unsafe fn cast_into<U: ObjcInstance>(self) -> StrongCell<U> {
        if DEBUG_MEMORY {
            println!("cast_into {} => {} {:p}",std::any::type_name::<T>(), std::any::type_name::<U>(), self.0.as_ptr());
        }
        let r = StrongCell::assume_retained(self.deref().cast());
        std::mem::forget(self);
        r
    }
}

impl<T: ObjcInstance> Clone for StrongCell<T> {
    fn clone(&self) -> Self {
        StrongCell::retaining(&self)
    }
}
impl<T: ObjcInstance> Drop for StrongCell<T> {
    fn drop(&mut self) {
        unsafe {
            if DEBUG_MEMORY {
                println!("Drop StrongCell<{}> {:p}",std::any::type_name::<T>(), self.0.as_ptr());
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
impl<T: PartialEq + ObjcInstance> PartialEq for StrongCell<T> {
    fn eq(&self, other: &Self) -> bool {
        let a: &T = self;
        let b: &T = other;
        a == b
    }
}
impl<T: Eq + ObjcInstance> Eq for StrongCell<T> {}
impl<T: Hash + ObjcInstance> Hash for StrongCell<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let a: &T = self;
        a.hash(state);
    }
}

//If the underlying objc instance is sync, we are Send
unsafe impl<T: ObjcInstance + Sync> Send for StrongCell<T> {}
///We are also Sync, because of the above situation and because ARC is threadsafe.
unsafe impl<T: ObjcInstance + Sync> Sync for StrongCell<T> {}

///Like StrongCell, but restricted to a particular lifetime.
///
/// This is typically used for objects that borrow some Rust data
#[derive(Debug)]
pub struct StrongLifetimeCell<'a, T: ObjcInstance>(NonNullImmutable<T>,PhantomData<&'a ()>);
impl<'a, T: ObjcInstance> StrongLifetimeCell<'a, T> {
    pub fn retaining(cell: &'a T) -> Self {
        unsafe {
            if DEBUG_MEMORY {
                println!("retain {} {:p}",std::any::type_name::<T>(), cell);
            }
            objc_retain(cell as *const T as *const c_void);
            Self::assume_retained_limited(cell)
        }
    }

    ///Converts to [AutoreleasedCell] by calling `autorelease` on `self`.
    ///
    ///Safe, but needs to be a moving function, because the StrongCell will not be valid once we
    /// decrement its reference counter.
    pub fn autoreleasing<'b: 'a>(cell: &'a Self, pool: &'b ActiveAutoreleasePool) -> AutoreleasedCell<'b, T> {
        AutoreleasedCell::autoreleasing(cell, pool)
    }
    ///Converts to [Self] by assuming the argument is already retained.
    ///
    /// This is usually the case for some objc methods with names like `new`, `copy`, `init`, etc.
    /// # Safety
    /// You are repsonsible to check:
    /// * That the type is retained
    /// * That the type can remain valid for the lifetime specified.  e.g., all "inner pointers" or "borrowed data" involved
    /// in this object will remain valid for the lifetime specified, which is unbounded.
    /// * That all objc APIs which end up seeing this pointer will either only access it for the lifetime specified,
    ///   or will take some other step (usually, copying) the object into a longer lifetime.
    pub unsafe fn assume_retained_limited(reference: &'a T) -> Self {
        if DEBUG_MEMORY {
            println!("assume_retained_limited {} {:p}",std::any::type_name::<T>(), reference);
        }
        StrongLifetimeCell(NonNullImmutable::from_reference(reference), PhantomData::default())
    }

    ///Reinterprets this cell as a cell of another type.
    ///
    /// # Performance
    /// This is a 0-cost abstraction.  The retain/release calls of converting to the new cell type are elided.
    /// # Safety
    /// You must comply with all the safety guarantees of [ObjcInstanceBehavior::cast].
    #[inline] pub unsafe fn cast_into<U: ObjcInstance + 'a>(self) -> StrongLifetimeCell<'a, U>{
        let r = StrongLifetimeCell::assume_retained_limited(&*(self.0.as_ptr() as *const U));
        std::mem::forget(self);
        r
    }
}

impl<'a, T: ObjcInstance> Drop for StrongLifetimeCell<'a, T> {
    fn drop(&mut self) {
        unsafe {
            if DEBUG_MEMORY {
                println!("Drop {} {:p}",std::any::type_name::<T>(), self);
            }
            objc_release(self.0.as_ptr() as *const _ as *const c_void);
        }
    }
}
impl<'a, T: ObjcInstance> std::ops::Deref for StrongLifetimeCell<'a, T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T {
        unsafe{ &*self.0.as_ptr()}
    }
}

impl<'a, T: ObjcInstance> std::fmt::Display for StrongLifetimeCell<'a, T> where T: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ptr = unsafe{ &*(self.0.as_ptr())};
        f.write_fmt(format_args!("{}",ptr))
    }
}
impl<'a, T: PartialEq + ObjcInstance> PartialEq for StrongLifetimeCell<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        let a: &T = self;
        let b: &T = other;
        a == b
    }
}
impl<'a, T: Eq + ObjcInstance> Eq for StrongLifetimeCell<'a, T> {}
impl<'a, T: Hash + ObjcInstance> Hash for StrongLifetimeCell<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let a: &T = self;
        a.hash(state);
    }
}

///[StrongCell], but mutable
#[derive(Debug)]
pub struct StrongMutCell<T: ObjcInstance>(NonNull<T>);
impl<T: ObjcInstance> StrongMutCell<T> {
    pub fn retaining(cell: &mut T) -> Self {
        if DEBUG_MEMORY {
            println!("retain {} {:p}",std::any::type_name::<T>(), cell);
        }
        unsafe {
            objc_retain(cell as *const T as *const c_void);
            Self::assume_retained(cell)
        }
    }

    ///Converts to [AutoreleasedCell] by calling `autorelease` on `self`.
    ///
    ///Safe, but needs to be a moving function, because the StrongCell will not be valid once we
    /// decrement its reference counter.
    pub fn autoreleasing<'a>(cell: &mut Self, pool: &'a ActiveAutoreleasePool) -> AutoreleasedMutCell<'a, T> {
        AutoreleasedMutCell::autoreleasing(cell, pool)
    }

    ///Converts to [StrongCell], e.g. dropping the mutable portion.
    ///
    /// This consumes the cell, e.g. you can't have an exclusive and nonexclusive reference to the same object.
    pub fn as_const(self) -> StrongCell<T> {
        let r: StrongCell<T> = unsafe{ StrongCell::assume_retained(&self) };
        std::mem::forget(self);
        r
    }

}

impl<T: ObjcInstance> StrongMutCell<T> {
    ///Converts to [Self] by assuming the argument is already retained.
    ///
    /// This is usually the case for some objc methods with names like `new`, `copy`, `init`, etc.
    /// # Safety
    /// If this isn't actually retained, will UB
    pub unsafe fn assume_retained(reference: &mut T) -> Self {
        if DEBUG_MEMORY {
            println!("assume_retained {} {:p}",std::any::type_name::<T>(), reference);
        }
        //safe because we're using a reference
        StrongMutCell(NonNull::new_unchecked(reference))
    }

    ///Attempts to use the "trampoline" trick to return an autoreleased value to objc.
    ///
    /// This is largely used when implementing a subclass.
    ///
    /// You must return the return value of this function, to your caller to get optimized results.
    /// Results are not guaranteed to be optimized, in part because inline assembly is not stabilized.
    #[inline(always)] pub fn return_autoreleased(self) -> *mut T {
        let ptr = self.0.as_ptr();
        std::mem::forget(self); //LEAK
        unsafe{ objc_autoreleaseReturnValue(ptr as *const c_void) as *const T as *mut T }
    }
    ///Reinterprets this cell as a cell of another type.
    ///
    /// # Performance
    /// This is a 0-cost abstraction.  The retain/release calls of converting to the new cell type are elided.
    /// # Safety
    /// You must comply with all the safety guarantees of [ObjcInstanceBehavior::cast].
    #[inline] pub unsafe fn cast_into<U: ObjcInstance>(mut self) -> StrongMutCell<U> {
        let r = StrongMutCell::assume_retained(self.deref_mut().cast_mut());
        std::mem::forget(self);
        r
    }
}

///We can implement Send for StrongMutCell conditional on T: Send, since we know the pointer is exclusive from StrongMutCell.
unsafe impl<T: ObjcInstance + Send> Send for StrongMutCell<T>  {}
///We can also implement sync if the type is sync.
unsafe impl<T: ObjcInstance + Sync> Sync for StrongMutCell<T> {}
impl<T: ObjcInstance> Drop for StrongMutCell<T> {
    fn drop(&mut self) {
        unsafe {
            if DEBUG_MEMORY {
                println!("Drop {} {:p}",std::any::type_name::<T>(), self);
            }
            objc_release(self.0.as_ptr() as *const _ as *const c_void);
        }
    }
}
impl<T: ObjcInstance> std::ops::Deref for StrongMutCell<T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T {
        unsafe{ &*self.0.as_ptr()}
    }
}
impl<T: ObjcInstance> std::ops::DerefMut for StrongMutCell<T> {
    #[inline] fn deref_mut(&mut self) -> &mut T {
        unsafe{ &mut *self.0.as_ptr()}
    }
}

impl<'a, T: ObjcInstance> std::fmt::Display for StrongMutCell<T> where T: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ptr = unsafe{ &*(self.0.as_ptr())};
        f.write_fmt(format_args!("{}",ptr))
    }
}
impl<T: PartialEq + ObjcInstance> PartialEq for StrongMutCell<T> {
    fn eq(&self, other: &Self) -> bool {
        let a: &T = self;
        let b: &T = other;
        a == b
    }
}
impl<T: Eq + ObjcInstance> Eq for StrongMutCell<T> {}
impl<T: Hash + ObjcInstance> Hash for StrongMutCell<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let a: &T = self;
        a.hash(state);
    }
}








