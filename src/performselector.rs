use std::ffi::c_void;
use super::arguments::{Arguments};
use super::arguments::Primitive;
use super::objectpointers::{AutoreleasedCell};
use super::sel::Sel;
use super::objcinstance::ObjcInstance;
use super::autorelease::ActiveAutoreleasePool;
use crate::bindings::{NSError,ObjcClass};
use crate::class::AnyClass;


///Types that can be performedSelector.
///
/// # Stability
/// Do not implement this type directly.  Instead use [crate::bindings::objc_instance!] or [crate::bindings::objc_class!].
///
/// # Safety
/// This requires the underlying type to be FFI-safe and a valid ObjC pointer.
///
//- not documentation
//This cannot be sealed because we intend it to be implemented on every ObjcInstance
pub unsafe trait PerformablePointer {}

//should be safe because ObjcInstance is FFI-safe
unsafe impl<O: ObjcInstance> PerformablePointer for O {}

///Trait where we can also call methods on super.  This requires knowing a superclass.
/// # Stability
/// Do not implement this type directly.  Instead use [crate::bindings::objc_instance!] or [crate::bindings::objc_class!].
///
/// # Safety
/// This requires the underlying type to be FFI-safe and a valid Objc pointer.
///
// - not documentation
//This cannot be sealed because we intend it to be implemented on every ObjCClass
pub unsafe trait PerformableSuper: PerformablePointer {
    fn any_class() -> &'static AnyClass;
}
//should be OK since ObjcClass is FFI-safe
unsafe impl <O: ObjcClass + 'static> PerformableSuper for O {
    fn any_class() -> &'static AnyClass {
        //safe because these are memory-compatible and we are downcasting
        unsafe{ std::mem::transmute(Self::class()) }
    }
}
#[link(name="objc",kind="dylib")]
extern {
    //https://clang.llvm.org/docs/AutomaticReferenceCounting.html#arc-runtime-objc-retainautoreleasedreturnvalue
    pub(crate) fn objc_retainAutoreleasedReturnValue(id: *const c_void) -> *mut c_void;
}


///Trait that provides `PerformSelector` implementations.  Autoimplelmented for `T: PerformablePointer`
///
/// # Stability
/// Do not implement this trait yourself.  Instead use [crate::bindings::objc_instance!] or [crate::bindings::objc_class!]
pub trait PerformsSelector  {
    ///Performs selector, returning a primitive type.
    /// # Safety
    /// See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_primitive<A: Arguments, R: Primitive>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> R;

    ///Performs, returning the specified [ObjcInstance].  You must coerce this into some type according to your knowledge of ObjC convention.
    unsafe fn perform<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const R;
    ///Performs, returning the result of the specified [ObjcInstance].  You must coerce this into some type according to your knowledge of ObjC convention.
    ///
    /// By convention, the error value is an autoreleased [NSError].
    ///
    ///# Safety
    ///See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_result<'a, A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Result<*const R, AutoreleasedCell<'a, NSError>>;

    ///Performs, calling a function of pattern `- (BOOL)example:(Parameter*)parameter... error:(NSError **)error;`
    ///
    /// If the method returns `YES`, it is assumed not to error, and `Ok(())` will be returned.
    /// If it returns `NO`, it is assumed to error, and `Err(...)` will be returned.
    ///
    /// By convention, the error value is an autoreleased [NSError].
    ///
    /// # Safety
    /// See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_bool_result<'a, A: Arguments>(receiver: *mut Self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Result<(),AutoreleasedCell<'a, NSError>>;

    ///Performs, returning the specified [ObjcInstance].
    ///
    /// This variant assumes 1) the calling convention is +0, 2) the type returned to you is +1.  The implementation
    /// knows a trick to perform this conversion faster than you can do it manually.
    ///# Safety
    ///See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_autorelease_to_retain<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const R;

    ///Performs, returning the specified [ObjcInstance].
    ///
    /// This variant assumes 1) the calling convention is +0, 2) the type returned to you is +1.  The implementation
    /// knows a trick to perform this conversion faster than you can do it manually.
    ///By convention, the error value is an autoreleased [NSError].
    ///# Safety
    ///See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_result_autorelease_to_retain<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Result<*const R, AutoreleasedCell<'_, NSError>>;
}

///implementation detail of perform_autorelease_to_strong_nonnull
/// written here to ensure tailcall optimization
///
/// # Safety
/// Issues include:
/// 1.  ptr argument is raw and we don't check anything
/// 2.  This function logically increments a reference count (may be elided at runtime)
///
/// Optimal performance of this function requires the compiler to do tailcall optimization.
/// Hopefully I've written it clearly enough for it to understand.
#[inline(always)] unsafe fn magic_retaining_trampoline<A: Arguments, R: ObjcInstance>(ptr: *mut c_void,selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const R {
    let c: *mut c_void = Arguments::invoke_primitive(ptr, selector,pool,args);
    objc_retainAutoreleasedReturnValue(c) as *const R
}
/// Variant of [magic_retaining_trampoline] for super.
/// # Safety
/// In addition to the issues of [magic_retaining_trampoline], there is no verification that you have passed the correct super_class.
#[inline(always)] unsafe fn magic_retaining_trampoline_super<A: Arguments, R: ObjcInstance>(ptr: *mut c_void,selector: Sel, pool: &ActiveAutoreleasePool, class: *const AnyClass, args: A) -> *const R {
    let c: *mut c_void = Arguments::invoke_primitive_super(ptr, selector,pool,class, args);
    objc_retainAutoreleasedReturnValue(c) as *const R
}

impl<T: PerformablePointer> PerformsSelector for T  {
    #[inline] unsafe fn perform_primitive<A: Arguments, R: Primitive>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> R {
        Arguments::invoke_primitive(receiver as *mut _, selector, pool,args)
    }

    #[inline] unsafe fn perform<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const R {
        Arguments::invoke(receiver as *mut c_void, selector, pool, args)
    }

    #[inline] unsafe fn perform_result<'a, A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Result<*const R, AutoreleasedCell<'a, NSError>> {
        Arguments::invoke_error(receiver as *mut c_void, selector, pool, args)
    }

    #[inline] unsafe fn perform_bool_result<'a, A: Arguments>(receiver: *mut Self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Result<(),AutoreleasedCell<'a, NSError>> {
        Arguments::invoke_error_bool(receiver as *mut c_void, selector, pool, args)
    }

    #[inline] unsafe fn perform_autorelease_to_retain<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const R {
        magic_retaining_trampoline(receiver as *mut c_void, selector, pool, args)

    }

    #[inline] unsafe fn perform_result_autorelease_to_retain<'a, A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Result<*const R, AutoreleasedCell<'a, NSError>> {
       Arguments::invoke_error_trampoline_strong(receiver as *mut c_void, selector, pool, args)
    }
}

///Variants of the perform functions that talk to `super` instead of `self`.  In general, this is supported on classes.
pub trait PerformsSelectorSuper {
    ///Performs selector, returning a primitive type.
    ///
    /// # Safety
    ///See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_super_primitive<A: Arguments, R: Primitive>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> R;

    ///Performs, returning the specified [ObjcInstance].  You must coerce this into some type according to your knowledge of ObjC convention.
    ///
    /// # Safety
    ///See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_super<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const R;
    ///Performs, returning the result of the specified [ObjcInstance].  You must coerce this into some type according to your knowledge of ObjC convention.
    ///
    /// By convention, the error value is an autoreleased [NSError].
    ///
    ///
    /// # Safety
    ///See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_super_result<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Result<*const R, AutoreleasedCell<'_, NSError>>;

    ///Performs, returning the specified [ObjcInstance].
    ///
    /// This variant assumes 1) the calling convention is +0, 2) the type returned to you is +1.  The implementation
    /// knows a trick to perform this conversion faster than you can do it manually.
    ///
    ///
    /// # Safety
    ///See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_super_autorelease_to_retain<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const R;

    ///Performs, returning the specified [ObjcInstance].
    ///
    /// This variant assumes 1) the calling convention is +0, 2) the type returned to you is +1.  The implementation
    /// knows a trick to perform this conversion faster than you can do it manually.
    ///By convention, the error value is an autoreleased [NSError].
    ///
    /// # Safety
    ///See the safety section of [crate::bindings::objc_instance!].
    unsafe fn perform_super_result_autorelease_to_retain<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Result<*const R, AutoreleasedCell<'_, NSError>>;

}

impl<T: PerformableSuper> PerformsSelectorSuper for T {
    #[inline] unsafe fn perform_super_primitive<A: Arguments, R: Primitive>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> R {
        Arguments::invoke_primitive_super(receiver as *mut c_void, selector, pool,Self::any_class(), args)
    }

    #[inline] unsafe fn perform_super<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const R {
        Arguments::invoke_super(receiver as *mut c_void, selector, pool, Self::any_class(), args)
    }

    #[inline] unsafe fn perform_super_result<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Result<*const R, AutoreleasedCell<'_, NSError>> {
        Arguments::invoke_error_trampoline_super(receiver as *mut c_void, selector, pool, Self::any_class(), args)
    }

    #[inline] unsafe fn perform_super_autorelease_to_retain<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const R {
        magic_retaining_trampoline_super(receiver as *mut c_void, selector, pool, Self::any_class(), args)
    }

    #[inline] unsafe fn perform_super_result_autorelease_to_retain<A: Arguments, R: ObjcInstance>(receiver: *mut Self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Result<*const R, AutoreleasedCell<'_, NSError>> {
        Arguments::invoke_error_trampoline_strong_super(receiver as *mut c_void, selector, pool, Self::any_class(), args)
    }
}

