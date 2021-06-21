//! Implements Marker and other raw runtime behavior

use std::ffi::{c_void};
use std::marker::PhantomData;
use super::performselector::{PerformablePointer};
use std::ptr::NonNull;
use std::convert::TryFrom;
use crate::bindings::{UnwrappedCell, ObjcInstance, AutoreleasedCell, ActiveAutoreleasePool, ObjcClass, AnyClass};
use crate::marker::Errors::UnwrappingNil;
use std::fmt::Formatter;
use crate::objectpointers::StrongCell;
use crate::performselector::PerformableSuper;

///Raw pointer.
///
/// This does not guarantee the pointer is non-nil and does not involve `ObjcInstance` drop.
/// Compare with [GuaranteedMarker] and [UnwrappedCell].
///
/// This type is `#[repr(transparent)]` and can be shoved directly into a C function.
#[repr(transparent)]
pub struct RawMarker<T: ?Sized> {
    ptr: *mut c_void,
    _marker: PhantomData<T>
}

impl<T> RawMarker<T> {
    ///Unsafe because creating pointers is unsafe in this library
    /// See the discussion in [objc_instance!]
    pub unsafe fn new(ptr: *mut c_void) -> Self {
        RawMarker { ptr:ptr, _marker: PhantomData::default() }
    }
    pub fn is_nil(&self) -> bool {
        self.ptr.is_null()
    }
    ///Unsafe becuse we don't check your assumption that this is nil
    pub unsafe fn assuming_nonnil(self) -> GuaranteedMarker<T> {
        GuaranteedMarker {
            ptr: NonNull::new_unchecked(self.ptr),
            _marker: PhantomData::default()
        }
    }
    ///# safety
    /// what are you going to do with this?
    pub unsafe fn ptr(&self) -> *mut c_void {
        self.ptr
    }
    pub fn nil() -> Self {
        RawMarker { ptr: std::ptr::null_mut(), _marker: PhantomData::default()}
    }
}
#[non_exhaustive]
#[derive(Debug)]
pub enum Errors {
    UnwrappingNil
}
impl std::fmt::Display for Errors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnwrappingNil => {
                write!(f,"Unwrapping nil")
            }
        }
    }
}
impl std::error::Error for Errors {}

impl<T> TryFrom<RawMarker<T>> for GuaranteedMarker<T> {
    type Error = Errors;
    fn try_from(value: RawMarker<T>) -> Result<Self,Errors> {
        GuaranteedMarker::try_unwrap(value)
    }
}
///Marker that is "guaranteed" not to be nil.  This lets us do certain optimizations like storing `Option<Marker<T>>` inline.
///
/// This type is `#[repr(transparent)]` and can be shoved directly into a C function.
///
/// See also [crate::bindings::UnwrappedCell].
#[repr(transparent)]
pub struct GuaranteedMarker<T: ?Sized> {
    ptr: NonNull<c_void>,
    _marker: PhantomData<T>
}

impl<T: ObjcInstance> From<GuaranteedMarker<T>> for UnwrappedCell<T> {
    fn from(marker: GuaranteedMarker<T>) -> Self {
        //I think this is no more or less safe than before
        unsafe { UnwrappedCell::new(marker) }
    }
}



impl<T> std::fmt::Debug for GuaranteedMarker<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(formatter, "GuaranteedMarker<{}>: {:p}",stringify!(T),self.ptr)
    }
}
impl<T> std::fmt::Debug for RawMarker<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(formatter, "RawMarker<{}>: {:p}",stringify!(T),self.ptr)
    }
}

impl<T> PerformablePointer for GuaranteedMarker<T> {
    unsafe fn ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}
//markers that refer to classes can also perform against super
impl<T: ObjcClass> PerformableSuper for GuaranteedMarker<T> {
    unsafe fn any_class(&self) -> AnyClass {
        T::class().as_anyclass()
    }
}

///Implemented by either marker
pub trait Marker<T> {
}
impl<T> Marker<T> for RawMarker<T> {}
impl<T> Marker<T> for GuaranteedMarker<T> {}




impl<T> GuaranteedMarker<T> {
    ///Creates a guaranteed marker tpe from your pointer.
    ///
    /// # Safety
    /// This is unsafe because of course it is.  We don't verify your
    /// pointer is valid, points to the object you think it does, is non-zero, or anything
    pub const unsafe fn new_unchecked(ptr: *mut c_void) -> Self {
        GuaranteedMarker { ptr: NonNull::new_unchecked(ptr), _marker: PhantomData }
    }

    #[inline] pub unsafe fn into_raw_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
    ///Unsafe because
    ///
    /// 1.  Pointer ops are generally unsafe (see documentation for `[objc_instance!()])
    pub unsafe fn unsafe_clone(&self) -> Self { GuaranteedMarker { ptr: self.ptr, _marker: PhantomData::default()}}

    ///Create a dangling (that is, invalid-to-use-but-not-UB-to-create) marker.
    ///
    /// This is almost never what you want, but maybe it is.
    ///
    /// # Safety
    /// Not technically UB to call this, but UB to do anything with it...
    pub unsafe fn dangling() -> Self {
        GuaranteedMarker { ptr: NonNull::dangling(), _marker: PhantomData::default() }
    }

    ///Tries to unwrap the [RawMarker] to a GuaranteedMarker, errors if nil.
    ///
    /// Also available as a `TryFrom` implementation.
    pub fn try_unwrap(raw_marker: RawMarker<T>) -> Result<Self,Errors> {
        if raw_marker.is_nil() {
            return Err(Errors::UnwrappingNil)
        }
        //was not nil above
        Ok(unsafe{ GuaranteedMarker::new_unchecked(raw_marker.ptr)} )
    }

    pub fn as_raw(&self) -> RawMarker<Self> {
        unsafe{ RawMarker::new(self.ptr()) }
    }

    ///Performs a duck-typed cast
    ///
    /// # Safety
    /// There is no guarantee that the type casted into is compatible with the instance.
    pub unsafe fn cast<R>(&self) -> GuaranteedMarker<R> {
        GuaranteedMarker::new_unchecked(self.ptr())
    }
}
impl<T: ObjcInstance> GuaranteedMarker<T> {
    pub unsafe fn assuming_retained(self) -> StrongCell<T> { StrongCell::assuming_retained(self) }
    pub unsafe fn assuming_autoreleased<'a> (self, pool: &'a ActiveAutoreleasePool) -> AutoreleasedCell<'a, T> { UnwrappedCell::new(self).assuming_autoreleased(pool) }
}

///`&RawMarker`, but reified into a type.
///
/// It is a common bug to pass some `&type` situation into one of the `.performSelector...` methods.
/// So we don't allow that by default.  However it is technically sometimes valid
/// in the rare event that objc really wants you to pass a reference.
///
/// This usually happens in the case of errors, which are a by-ref in-out pattern
/// (e.g. `error:(__autoreleasing NSError **)error`).
///
/// You can use this explicit type if that is what you mean to do.
#[repr(transparent)]
#[derive(Debug)]
pub struct RawMarkerMutRef<'a, T>(&'a mut RawMarker<T>);

impl<'a, T> RawMarkerMutRef<'a,T> {
    pub fn from_marker(marker: &'a mut RawMarker<T>) -> Self {
        RawMarkerMutRef(marker)
    }
}