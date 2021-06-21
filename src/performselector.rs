use std::ffi::c_void;
use super::arguments::{Arguments};
use super::arguments::Primitive;
use super::objectpointers::{StrongCell, UnwrappedCell, AutoreleasedCell};
use super::sel::Sel;
use super::objcinstance::ObjcInstance;
use super::autorelease::ActiveAutoreleasePool;
use std::os::raw::c_char;
use crate::marker::{GuaranteedMarker,RawMarker};
use std::convert::TryInto;
use crate::bindings::NSError;
use crate::class::AnyClass;

///Types that can be performedSelector.
///
/// Examples include `Marker`, `Class`, and related pointer types.
pub trait PerformablePointer {
    unsafe fn ptr(&self) -> *mut c_void;
}
///Trait where we can also call methods on super.  This requires knowing a superclass.
pub trait PerformableSuper: PerformablePointer {
    unsafe fn any_class(&self) -> AnyClass;
}
#[link(name="objc",kind="dylib")]
extern {
    //https://clang.llvm.org/docs/AutomaticReferenceCounting.html#arc-runtime-objc-retainautoreleasedreturnvalue
    pub(crate) fn objc_retainAutoreleasedReturnValue(id: *mut c_void) -> *mut c_void;
}

///Certain private details of PerformsSelector
pub(crate) trait PerformsSelectorPrivate {
    unsafe fn perform_unmanaged_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> GuaranteedMarker<R>;
    unsafe fn perform_unmanaged_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Option<GuaranteedMarker<R>>;
}
impl<T: PerformablePointer> PerformsSelectorPrivate for T {
    #[inline] unsafe fn perform_unmanaged_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> GuaranteedMarker<R> {
        let raw_marker: RawMarker<R> = Arguments::invoke_marker(self.ptr(), selector, pool,args);
        raw_marker.assuming_nonnil()
    }
    #[inline] unsafe fn perform_unmanaged_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Option<GuaranteedMarker<R>> {
        let raw_marker: RawMarker<R> = Arguments::invoke_marker(self.ptr(), selector,pool,args);
        raw_marker.try_into().ok()
    }
}

///Trait that provides `PerformSelector` implementations.  Autoimplelmented for `T: PerformablePointer`
pub trait PerformsSelector {
    ///Performs selector, returning a primitive type.
    /// # Safety
    /// It's UB to call anything that throws, anything that isn't a valid selector for the type,
    unsafe fn perform_primitive<A: Arguments, R: Primitive>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> R;

    ///Performs selector, returning a primitive pointer assumed to be an inner memory of the objc object.
    ///
    /// # Safety
    /// If the underlying object is deallocated, the pointer will be invalid.  see `NS_RETURNS_INNER_POINTER` and/or `objc_returns_inner_pointer` for a discussion of this pattern.
    unsafe fn perform_inner_ptr<A: Arguments>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const c_char;
    ///Performs selector, assuming the return type is owned and not `nil` (like `init`)
    unsafe fn perform_owned_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> StrongCell<R>;
    ///Performs selector, assuming the return type is owned *if* it is non-`nil` (like a failable initializer)
    unsafe fn perform_owned_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Option<StrongCell<R>>;
    ///Performs selector, assuming the returntype is an autoreleased convention and converts to a strong pointer.  Assumes pointer is non-nil.
    ///
    ///This implementation is fucking magic, and under-the-hood uses `objc_retainAutoreleasedReturnValue`.
    /// See https://www.mikeash.com/pyblog/friday-qa-2011-09-30-automatic-reference-counting.html for details, but effectively
    /// 1.  We assume the convention is 'unowned'
    /// 2.  In practice, a caller that understands our magic will elide the `autorelease` *at runtime*
    /// This is a performance trick the objc compiler uses which we replicate for common cases.
    ///
    /// # Safety
    /// Assumes pointer is non-nil and that the receiver is valid and responds to selector
    unsafe fn perform_autorelease_to_strong_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A)  -> StrongCell<R>;

    ///Performs selector, assuming the returntype is an autoreleased convention and converts to a strong pointer.  Handles nullable case.
    ///
    ///This implementation is fucking magic, and under-the-hood uses `objc_retainAutoreleasedReturnValue`.
    /// See https://www.mikeash.com/pyblog/friday-qa-2011-09-30-automatic-reference-counting.html for details, but effectively
    /// 1.  We assume the convention is 'unowned'
    /// 2.  In practice, a caller that understands our magic will elide the `autorelease` *at runtime*
    /// This is a performance trick the objc compiler uses which we replicate for common cases.
    ///
    /// # Safety
    /// the receiver is valid and responds to selector
    unsafe fn perform_autorelease_to_strong_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A)  -> Option<StrongCell<R>>;

    ///Performs selector, assuming the return type is part of an autorelease pool and is non-`nil`.
    unsafe fn perform_autoreleased_nonnull<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> AutoreleasedCell<'a, R>;
    ///Performs selector, assuming the return type is part of an autorelease pool *if* it is non-`nil`.
    unsafe fn perform_autoreleased_nullable<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Option<AutoreleasedCell<'a, R>>;

    ///Performs selector, assuming
    /// * a trailing error argument
    /// * return value is autoreleased and wants strong conversion
    ///
    unsafe fn perform_autoreleased_to_strong_error<'a, A: Arguments, R: ObjcInstance> (&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Result<StrongCell<R>, AutoreleasedCell<'a, NSError>>;
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
#[inline(always)] unsafe fn magic_retaining_trampoline<A: Arguments>(ptr: *mut c_void,selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *mut c_void {
    let c: *mut c_void = Arguments::invoke_primitive(ptr, selector,pool,args);
    objc_retainAutoreleasedReturnValue(c)
}
/// Variant of [magic_retaining_trampoline] for super.
/// # Safety
/// In addition to the issues of [magic_retaining_trampoline], there is no verification that you have passed the correct super_class.
#[inline(always)] unsafe fn magic_retaining_trampoline_super<A: Arguments>(ptr: *mut c_void,selector: Sel, pool: &ActiveAutoreleasePool, class: AnyClass, args: A) -> *mut c_void {
    let c: *mut c_void = Arguments::invoke_primitive_super(ptr, selector,pool,class, args);
    objc_retainAutoreleasedReturnValue(c)
}

impl<T: PerformablePointer> PerformsSelector for T  {
    #[inline] unsafe fn perform_primitive<A: Arguments, R: Primitive>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> R {
        Arguments::invoke_primitive(self.ptr(), selector, pool,args)
    }

    #[inline] unsafe fn perform_inner_ptr<A: Arguments>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const c_char {
        Arguments::invoke_primitive(self.ptr(), selector, pool,args)
    }
    #[inline] unsafe fn perform_owned_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> StrongCell<R> {
        let raw_marker: RawMarker<R> = Arguments::invoke_marker(self.ptr(), selector,pool,args);
        let cell: UnwrappedCell<R> = raw_marker.assuming_nonnil().into();
        cell.assuming_retained()
    }

    #[inline] unsafe fn perform_owned_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Option<StrongCell<R>> {
        self.perform_unmanaged_nullable(selector, pool,args).map(|u| {
            let k: UnwrappedCell<R> = UnwrappedCell::new(u);
            k.assuming_retained()
        })
    }

    #[inline] unsafe fn perform_autorelease_to_strong_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A)  -> StrongCell<R> {
        let ptr = magic_retaining_trampoline(self.ptr(),selector,pool,args);
        let raw_marker: RawMarker<R> = RawMarker::new(ptr);
        let cell: UnwrappedCell<R> = raw_marker.assuming_nonnil().into();
        cell.assuming_retained()
    }

    #[inline] unsafe fn perform_autorelease_to_strong_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel,pool: &ActiveAutoreleasePool, args: A)  -> Option<StrongCell<R>> {
        //+1
        let ptr = magic_retaining_trampoline(self.ptr(), selector,pool, args);
        let raw_marker: RawMarker<R> = RawMarker::new(ptr);
        let guaranted_marker: Option<GuaranteedMarker<R>> = raw_marker.try_into().ok();
        let r: Option<StrongCell<R>> = guaranted_marker.map(|c|
            UnwrappedCell::new(c).assuming_retained()
        );
        r
    }


    #[inline] unsafe fn perform_autoreleased_nonnull<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> AutoreleasedCell<'a, R> {
        let raw_marker: RawMarker<R> = Arguments::invoke_marker(self.ptr(), selector,pool,args);
        UnwrappedCell::new(raw_marker.assuming_nonnil()).assuming_autoreleased(pool)
    }
    ///Performs selector, assuming the return type is part of an autorelease pool *if* it is non-`nil`.
    #[inline] unsafe fn perform_autoreleased_nullable<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Option<AutoreleasedCell<'a, R>> {
        self.perform_unmanaged_nullable(selector, pool,args).map(|u| (UnwrappedCell::<R>::new(u)).assuming_autoreleased(pool))
    }

    #[inline] unsafe fn perform_autoreleased_to_strong_error<'a, A: Arguments, R: ObjcInstance> (&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Result<StrongCell<R>, AutoreleasedCell<'a, NSError>> {
        //this one is complex; we implemented it inside the arguments instead for inlineability and ensuring we get the correct machinecode output
        Arguments::invoke_error_trampoline_strong(self.ptr(), selector, pool, args).map_err(|e| UnwrappedCell::new(e).assuming_autoreleased(pool))
    }

}

///Variants of the perform functions that talk to `super` instead of `self`.  In general, this is supported on classes.
pub trait PerformsSelectorSuper {
    ///Performs selector, returning a primitive type.
    /// # Safety
    /// It's UB to call anything that throws, anything that isn't a valid selector for the type,
    unsafe fn perform_super_primitive<A: Arguments, R: Primitive>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> R;

    ///Performs selector, returning a primitive pointer assumed to be an inner memory of the objc object.
    ///
    /// # Safety
    /// If the underlying object is deallocated, the pointer will be invalid.  see `NS_RETURNS_INNER_POINTER` and/or `objc_returns_inner_pointer` for a discussion of this pattern.
    unsafe fn perform_super_inner_ptr<A: Arguments>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const c_char;
    ///Performs selector, assuming the return type is owned and not `nil` (like `init`)
    unsafe fn perform_super_owned_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> StrongCell<R>;
    ///Performs selector, assuming the return type is owned *if* it is non-`nil` (like a failable initializer)
    unsafe fn perform_super_owned_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Option<StrongCell<R>>;
    ///Performs selector, assuming the returntype is an autoreleased convention and converts to a strong pointer.  Assumes pointer is non-nil.
    ///
    ///This implementation is fucking magic, and under-the-hood uses `objc_retainAutoreleasedReturnValue`.
    /// See https://www.mikeash.com/pyblog/friday-qa-2011-09-30-automatic-reference-counting.html for details, but effectively
    /// 1.  We assume the convention is 'unowned'
    /// 2.  In practice, a caller that understands our magic will elide the `autorelease` *at runtime*
    /// This is a performance trick the objc compiler uses which we replicate for common cases.
    ///
    /// # Safety
    /// Assumes pointer is non-nil and that the receiver is valid and responds to selector
    unsafe fn perform_super_autorelease_to_strong_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A)  -> StrongCell<R>;

    ///Performs selector, assuming the returntype is an autoreleased convention and converts to a strong pointer.  Handles nullable case.
    ///
    ///This implementation is fucking magic, and under-the-hood uses `objc_retainAutoreleasedReturnValue`.
    /// See https://www.mikeash.com/pyblog/friday-qa-2011-09-30-automatic-reference-counting.html for details, but effectively
    /// 1.  We assume the convention is 'unowned'
    /// 2.  In practice, a caller that understands our magic will elide the `autorelease` *at runtime*
    /// This is a performance trick the objc compiler uses which we replicate for common cases.
    ///
    /// # Safety
    /// the receiver is valid and responds to selector
    unsafe fn perform_super_autorelease_to_strong_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A)  -> Option<StrongCell<R>>;

    ///Performs selector, assuming the return type is part of an autorelease pool and is non-`nil`.
    unsafe fn perform_super_autoreleased_nonnull<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> AutoreleasedCell<'a, R>;
    ///Performs selector, assuming the return type is part of an autorelease pool *if* it is non-`nil`.
    unsafe fn perform_super_autoreleased_nullable<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Option<AutoreleasedCell<'a, R>>;

    ///Performs selector, assuming
    /// * a trailing error argument
    /// * return value is autoreleased and wants strong conversion
    ///
    unsafe fn perform_super_autoreleased_to_strong_error<'a, A: Arguments, R: ObjcInstance> (&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Result<StrongCell<R>, AutoreleasedCell<'a, NSError>>;
    ///Performs selector against super, assuming
    /// * return value is nonnull
    /// * No memory management on return value
    unsafe fn perform_super_unmanaged_nonnull<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> UnwrappedCell<R>;
}

impl<T: PerformableSuper> PerformsSelectorSuper for T {
    #[inline] unsafe fn perform_super_primitive<A: Arguments, R: Primitive>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> R {
        Arguments::invoke_primitive_super(self.ptr(), selector, pool,self.any_class(), args)
    }

    #[inline] unsafe fn perform_super_inner_ptr<A: Arguments>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> *const c_char {
        Arguments::invoke_primitive_super(self.ptr(), selector, pool,self.any_class(), args)
    }
    #[inline] unsafe fn perform_super_owned_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> StrongCell<R> {
        let raw_marker: RawMarker<R> = Arguments::invoke_marker_super(self.ptr(), selector,pool,self.any_class(), args);
        let cell: UnwrappedCell<R> = raw_marker.assuming_nonnil().into();
        cell.assuming_retained()
    }

    #[inline] unsafe fn perform_super_owned_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A) -> Option<StrongCell<R>> {
        let raw_marker: RawMarker<R> = Arguments::invoke_marker_super(self.ptr(), selector,pool,self.any_class(), args);
        raw_marker.try_into().ok().map(|n| UnwrappedCell::new(n).assuming_retained())
    }

    #[inline] unsafe fn perform_super_autorelease_to_strong_nonnull<A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &ActiveAutoreleasePool, args: A)  -> StrongCell<R> {
        let ptr = magic_retaining_trampoline_super(self.ptr(),selector,pool,self.any_class(), args);
        let raw_marker: RawMarker<R> = RawMarker::new(ptr);
        let cell: UnwrappedCell<R> = raw_marker.assuming_nonnil().into();
        cell.assuming_retained()
    }

    #[inline] unsafe fn perform_super_autorelease_to_strong_nullable<A: Arguments, R: ObjcInstance>(&self, selector: Sel,pool: &ActiveAutoreleasePool, args: A)  -> Option<StrongCell<R>> {
        //+1
        let ptr = magic_retaining_trampoline_super(self.ptr(), selector,pool, self.any_class(), args);
        let raw_marker: RawMarker<R> = RawMarker::new(ptr);
        let guaranted_marker: Option<GuaranteedMarker<R>> = raw_marker.try_into().ok();
        let r: Option<StrongCell<R>> = guaranted_marker.map(|c|
            UnwrappedCell::new(c).assuming_retained()
        );
        r
    }


    #[inline] unsafe fn perform_super_autoreleased_nonnull<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> AutoreleasedCell<'a, R> {
        let raw_marker: RawMarker<R> = Arguments::invoke_marker_super(self.ptr(), selector,pool,self.any_class(), args);
        UnwrappedCell::new(raw_marker.assuming_nonnil()).assuming_autoreleased(pool)
    }
    ///Performs selector, assuming the return type is part of an autorelease pool *if* it is non-`nil`.
    #[inline] unsafe fn perform_super_autoreleased_nullable<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Option<AutoreleasedCell<'a, R>> {
        let raw_marker: RawMarker<R> = Arguments::invoke_marker_super(self.ptr(), selector,pool,self.any_class(), args);
        raw_marker.try_into().ok().map(|u| UnwrappedCell::<R>::new(u).assuming_autoreleased(pool))
    }

    #[inline] unsafe fn perform_super_autoreleased_to_strong_error<'a, A: Arguments, R: ObjcInstance> (&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> Result<StrongCell<R>, AutoreleasedCell<'a, NSError>> {
        //this one is complex; we implemented it inside the arguments instead for inlineability and ensuring we get the correct machinecode output
        Arguments::invoke_error_trampoline_strong_super(self.ptr(), selector, pool, self.any_class(), args).map_err(|e| UnwrappedCell::new(e).assuming_autoreleased(pool))
    }
    #[inline] unsafe fn perform_super_unmanaged_nonnull<'a, A: Arguments, R: ObjcInstance>(&self, selector: Sel, pool: &'a ActiveAutoreleasePool, args: A) -> UnwrappedCell<R> {
        UnwrappedCell::new(Arguments::invoke_marker_super(self.ptr(), selector, pool, self.any_class(), args).assuming_nonnil())
    }

}

