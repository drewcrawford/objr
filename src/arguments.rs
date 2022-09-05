///!Rust doesn't natively support varargs, so encoding the args
///!into an "anonymous" type that implements this trait is a convenient
///! way to pass the objcargs to functions.

use super::bindings::*;
use std::ffi::c_void;
use std::fmt::Debug;
use std::mem::size_of;

#[link(name="objc", kind="dylib")]
extern "C" {
    fn objc_msgSend();
    fn objc_msgSend_stret();
    //Undocumented, but part of ABI.  This call goes directly to super.  Do not pass go, do not try `self`.
    fn objc_msgSendSuper2();
    fn objc_msgSendSuper2_stret();
}

//defined in https://opensource.apple.com/source/objc4/objc4-371.2/runtime/message.h
//This is the first argument to `objc_msgSendSuper2` instead of the receiver
#[repr(C)]
struct ObjcSuper {
    receiver: *mut c_void,
    /* Although the "documentation" says that "super_class is the first class to search"
     in fact when calling `objc_msgSendSuper2` you want to pass the class of the receiver here
     (e.g, not the class to search).

     This is probably a quirk of objc_msgSendSuper2.
     */
    class: *const AnyClass,
}

///Trait describing a type that can be used as arugments.  Generally, this is a tuple of all the arguments to some method.
///
/// This type is sealed; you may not implement it from outside the crate.
/// All implementations are provided via macro.
pub trait Arguments: Sized + Debug + crate::private::Sealed {
    ///Implementation deatil of [PerformsSelector::perform_primitive]
    unsafe fn invoke_primitive<R: Primitive>(receiver: *mut c_void, sel: Sel, pool: &ActiveAutoreleasePool, args: Self) -> R;
    ///Implementation detail of [PerformsSelectorSuper::perform_super_primitive]
    unsafe fn invoke_primitive_super<R: Primitive>(obj: *mut c_void, sel: Sel, _pool: &ActiveAutoreleasePool, class: *const AnyClass, args: Self) -> R;
    ///Implementation detail of [PerformsSelector::perform]
    unsafe fn invoke<R: ObjcInstance>(receiver: *mut c_void, sel: Sel, pool: &ActiveAutoreleasePool, args: Self) -> *const R;
    ///Implementation detail of [PerformsSelectorSuper::perform_super]
    unsafe fn invoke_super<R: ObjcInstance>(receiver: *mut c_void, sel: Sel, pool: &ActiveAutoreleasePool, class: *const AnyClass,args: Self) -> *const R;
    ///Implementation detail of [PerformsSelector::perform_result]
    unsafe fn invoke_error<'a, R: ObjcInstance>(receiver: *mut c_void, sel: Sel, pool: &'a ActiveAutoreleasePool, args: Self) -> Result<*const R, AutoreleasedCell<'a, NSError>>;
    ///Implementation detail of [PerformsSelector::perform_bool_result].
    unsafe fn invoke_error_bool<'a>(receiver: *mut c_void, sel: Sel, pool: &'a ActiveAutoreleasePool, args: Self) -> Result<(), AutoreleasedCell<'a, NSError>>;
    ///Implementation detail of [PerformablePointer::perform_result_autorelease_to_retain]
    unsafe fn invoke_error_trampoline_strong<'a, R: ObjcInstance>(obj: *mut c_void, sel: Sel, _pool: &'a ActiveAutoreleasePool, args: Self) -> Result<*const R,AutoreleasedCell<'a, NSError>>;
    ///Implementation detail of [PerformsSelectorSuper::perform_super_result_autorelease_to_retain]
    unsafe fn invoke_error_trampoline_strong_super<'a, R: ObjcInstance>(obj: *mut c_void, sel: Sel, _pool: &'a ActiveAutoreleasePool, class: *const AnyClass, args: Self) -> Result<*const R,AutoreleasedCell<'a, NSError>>;
    ///Implementation detail of [PerformsSelectorSuper::perform_super_autorelease_to_retain]
    unsafe fn invoke_error_trampoline_super<'a, R: ObjcInstance>(receiver: *mut c_void, sel: Sel, pool: &'a ActiveAutoreleasePool, class: *const AnyClass, args: Self) -> Result<*const R, AutoreleasedCell<'a, NSError>>;
}

///Can be used as an argument in objr
///
/// This constraint provides additional safety around transmuting fp types.
///
/// # Safety
/// The primary constraint of this protocol is it needs to be FFI-safe (`#[repr(transparent)]` or `#[repr(C)]`).
/// Since this cannot be otherwise verified, we're going to declare it `unsafe`.
/// # See also
/// [Primitive], which implies this trait. The difference is that [Arguable] does not allow the [PerformsSelector::perform_primitive()]
/// family in its return type.
pub unsafe trait Arguable  {}

/*
Brief explanation of this type design.

Because of negative trait bounds, we can't declare blanket impls for both ObjcInstance and Primitive
(neither are sealed, and a type could in theory be both.)

Instead we implement Arguable on the wrapping type (such as the objc wrapping type).
These cannot normally be constructed.

The arguable part is only the exclusive (e.g. mutable) pointers.  Non-exclusive pointers
must be opted in with [ArguableBehavior::assume_nonmut_perform()]
 */
//primitive types can have exclusive references passed
unsafe impl<O: Arguable> Arguable for &mut O {}
unsafe impl<O: Arguable> Arguable for *mut O {}


pub trait ArguableBehavior {
    type Target;
    ///Allows you to call [objr::bindings::PerformsSelector::perform] from a nonmutating context.
    ///
    /// This function should not be used for general-purpose pointer casting.
    ///
    /// # Safety
    /// This is only safe when the underlying objc method does not mutate the receiver.  See [objc_instance#Mutability] for details.
    unsafe fn assume_nonmut_perform(self) -> Self::Target;
}

impl<A: Arguable> ArguableBehavior for *const A {
    type Target = *mut A;
    unsafe fn assume_nonmut_perform(self) -> Self::Target {
        self as *mut A
    }
}

impl<A: Arguable> ArguableBehavior for &A {
    type Target = *mut A;
    unsafe fn assume_nonmut_perform(self) -> Self::Target {
        self as *const A as *mut A
    }
}
impl<A: Arguable> ArguableBehavior for Option<&A> {
    type Target = *mut A;
    unsafe fn assume_nonmut_perform(self) -> Self::Target {
        self.map(|x| x as *const A as *mut A).unwrap_or(std::ptr::null_mut())
    }
}


///Non-reference types that are ObjC FFI-safe.  This marker
/// allows access to the [PerformsSelector::perform_primitive()] family.
///
/// # Safety
/// Type must be FFI-safe.
///
/// # Note
/// This is unsealed because we want to allow structs to be declared as primitives in external crates.
///
/// # See also
///  This used to inherit from [Arguable], but now they are distinct.  [Primitive] means it can appear as a return value,
/// whereas [Arguable] can appear as an argument.  Generally speaking, parameters must be `mut` (or stepped up with [ArguableBehavior::assume_nonmut_perform],
/// whereas return types can be const.
///
/// This cannot inherit from Arguable because various types are primitives (for example, `*const Struct`) but we only allow arguing `*mut Struct`.
pub unsafe trait Primitive: Sized {
}

unsafe impl<P: Primitive> Primitive for *const P {}


//This is safe because these are all ffi-safe.
unsafe impl Primitive for Sel {}
unsafe impl Arguable for Sel {}

unsafe impl Primitive for bool{}
unsafe impl Arguable for bool{}

unsafe impl Primitive for *mut c_void {}
unsafe impl Arguable for *mut c_void {}

unsafe impl Primitive for *const c_void {}
unsafe impl Arguable for *const c_void {}

unsafe impl Primitive for f64 {}
unsafe impl Arguable for f64 {}

unsafe impl Primitive for () {}
unsafe impl Arguable for () {}

unsafe impl Primitive for u64{}
unsafe impl Arguable for u64{}
unsafe impl Primitive for u32{}
unsafe impl Arguable for u32{}
unsafe impl Primitive for u16{}
unsafe impl Arguable for u16{}
unsafe impl Primitive for u8{}
unsafe impl Arguable for u8{}



unsafe impl Arguable for i64 {}
unsafe impl Primitive for i64 {}
unsafe impl Arguable for i32 {}
unsafe impl Primitive for i32 {}
unsafe impl Arguable for i16 {}
unsafe impl Primitive for i16 {}
unsafe impl Arguable for i8 {}
unsafe impl Primitive for i8 {}

///Implementation macro for declaring [Argument] types.
macro_rules! arguments_impl {
    (
        $($identifier:ident : $type:ident),*
    ) => (
        //seal the type
        impl<$($type:Arguable),*> crate::objr::private::Sealed for ($($type,)*) where $($type: Debug),* {}
        impl<$($type:Arguable),*> Arguments for ($($type,)*) where $($type: Debug),* {
           #[inline] unsafe fn invoke_primitive<R: Primitive>(obj: *mut c_void, sel: Sel, _pool: &ActiveAutoreleasePool, ($($identifier,)*): Self) -> R {
               //autoreleasepool is encouraged by signature but not used

                let impcast = if cfg!(target_arch="x86_64") {
                    //this condition seems to broadly agree with clang
                    if size_of::<R>() <= 16 {
                        objc_msgSend
                    }
                    else {
                        objc_msgSend_stret
                    }
                    /*NOTE: For "long double" we need fpret, but there does not seem to be an equivalent rust type.

                    In general there isn't a type on apple silicon either, I think this is not widely used by the runtime and so it can
                    be ignored.
                   */
                }
                else {
                    objc_msgSend
                };
                let imp: unsafe extern fn(*mut c_void, Sel $(, $type)*) -> R =
                    std::mem::transmute(impcast);
                imp(obj, sel $(, $identifier)*)
            }
           #[inline] unsafe fn invoke_primitive_super<R: Primitive>(obj: *mut c_void, sel: Sel, _pool: &ActiveAutoreleasePool, class: *const AnyClass, ($($identifier,)*): Self) -> R {
               let objc_super = ObjcSuper {
                   receiver: obj,
                   class: class
               };
               let impcast = if cfg!(target_arch="x86_64") {
                    //this condition seems to broadly agree with clang
                    if size_of::<R>() <= 16 {
                        objc_msgSendSuper2
                    }
                    else {
                        objc_msgSendSuper2_stret
                    }
                    /*NOTE: I verified in clang that, for "long double" case, we still use objc_msgSendSuper2.  I have no explanation
                    for why there is no fpret verison.  However since we don't deal with fpret anyway, this is somewhat irrelevant.
                     */
                }
                else {
                    objc_msgSendSuper2
                };
                let imp: unsafe extern fn(*const ObjcSuper, Sel $(, $type)*) -> R =
                    std::mem::transmute(impcast);
                imp(&objc_super, sel $(, $identifier)*)
            }
            #[inline] unsafe fn invoke<R: ObjcInstance>(obj: *mut c_void, sel: Sel, _pool: &ActiveAutoreleasePool, ($($identifier,)*): Self) -> *const R {
               //autoreleasepool is encouraged by signature but not used
               let impcast = objc_msgSend as unsafe extern fn();
                let imp: unsafe extern fn(*mut c_void, Sel $(, $type)*) -> *mut c_void =
                    std::mem::transmute(impcast);
                let ptr = imp(obj, sel $(, $identifier)*);
                ptr as *const R
            }
           #[inline] unsafe fn invoke_super<R: ObjcInstance>(obj: *mut c_void, sel: Sel, _pool: &ActiveAutoreleasePool,class: *const AnyClass, ($($identifier,)*): Self) -> *const R {
               let objc_super = ObjcSuper {
                   receiver: obj,
                   class: class
               };
               let impcast = objc_msgSendSuper2 as unsafe extern fn();
                let imp: unsafe extern "C" fn(*const ObjcSuper, Sel $(, $type)*) -> *mut c_void =
                    std::mem::transmute(impcast);
                let ptr = imp(&objc_super, sel $(, $identifier)*);
                ptr as *const R
            }

           ///This function combines various common behaviors in a fast implementation.
           /// In particular I want to make sure we generate the right machinecode for `objc_retainAutoreleasedReturnValue`
           ///
           /// 1.  Invoke / performSelector
           /// 2.  Assumes trailing error parameter
           /// 3.  Caller wants +1 / StrongCell, but callee returns +0 / autoreleased.  Resolved via the magic trampoline `objc_retainAutoreleasedReturnValue`.
           ///
            #[inline] unsafe fn invoke_error_trampoline_strong<'a, R: ObjcInstance>(obj: *mut c_void, sel: Sel, pool: &'a ActiveAutoreleasePool, ($($identifier,)*): Self) -> Result<*const R,AutoreleasedCell<'a, NSError>> {
               use crate::performselector::objc_retainAutoreleasedReturnValue;
               let impcast = objc_msgSend as unsafe extern fn();
               let mut error: *const NSError = std::ptr::null();
               let imp: unsafe extern fn(*mut c_void, Sel, $( $type, )* &mut *const NSError) -> *const R  = std::mem::transmute(impcast);
               let ptr = imp(obj,sel, $($identifier,)* &mut error );
               //ok to call this with nil
               objc_retainAutoreleasedReturnValue(ptr as *const c_void);
               if ptr != std::ptr::null_mut() {
                   Ok(ptr)
               }
               else {
                   //I'm pretty sure it's street-legal to assume this
                   //although if it's not, don't sue me
                   Err(NSError::assume_nonnil(error).assume_autoreleased(pool))
               }
           }
           #[inline] unsafe fn invoke_error<'a, R: ObjcInstance>(receiver: *mut c_void, sel: Sel, pool: &'a ActiveAutoreleasePool, ($($identifier,)*): Self) -> Result<*const R, AutoreleasedCell<'a, NSError>> {
               let impcast = objc_msgSend as unsafe extern fn();
               let mut error: *const NSError = std::ptr::null();
               let imp: unsafe extern fn(*mut c_void, Sel, $( $type, )* &mut *const NSError) -> *const R  = std::mem::transmute(impcast);
               let ptr = imp(receiver,sel, $($identifier,)* &mut error );
               if ptr != std::ptr::null_mut() {
                   Ok(ptr)
               }
               else {
                   //I'm pretty sure it's street-legal to assume this
                   //although if it's not, don't sue me
                   Err(NSError::assume_nonnil(error).assume_autoreleased(pool))
               }
           }
           #[inline] unsafe fn invoke_error_bool<'a>(receiver: *mut c_void, sel: Sel, pool: &'a ActiveAutoreleasePool, ($($identifier,)*): Self) -> Result<(), AutoreleasedCell<'a, NSError>> {
               let impcast = objc_msgSend as unsafe extern fn();
               let mut error: *const NSError = std::ptr::null();
               let imp: unsafe extern fn(*mut c_void, Sel, $( $type, )* &mut *const NSError) -> bool  = std::mem::transmute(impcast);
               let r = imp(receiver,sel, $($identifier,)* &mut error );
               if r {
                   Ok(())
               }
               else {
                   //I'm pretty sure it's street-legal to assume this
                   //although if it's not, don't sue me
                   Err(NSError::assume_nonnil(error).assume_autoreleased(pool))
               }
           }


           #[inline] unsafe fn invoke_error_trampoline_strong_super<'a, R: ObjcInstance>(obj: *mut c_void, sel: Sel, pool: &'a ActiveAutoreleasePool, class: *const AnyClass, ($($identifier,)*): Self) -> Result<*const R,AutoreleasedCell<'a, NSError>> {
               let objc_super = ObjcSuper {
                   receiver: obj,
                   class: class
               };
               use crate::performselector::objc_retainAutoreleasedReturnValue;
               let impcast = objc_msgSendSuper2 as unsafe extern fn();
               let mut error: *const NSError = std::ptr::null();
               let imp: unsafe extern fn(*const ObjcSuper, Sel, $( $type, )* &mut *const NSError) -> *const R  = std::mem::transmute(impcast);
               let ptr = imp(&objc_super,sel, $($identifier,)* &mut error );
               //ok to call this with nil
               objc_retainAutoreleasedReturnValue(ptr as *const c_void);
               if ptr != std::ptr::null_mut() {
                   Ok(ptr)
               }
               else {
                   //I'm pretty sure it's street-legal to assume this
                   //although if it's not, don't sue me
                   Err(NSError::assume_nonnil(error).assume_autoreleased(pool))
               }

           }
           #[inline] unsafe fn invoke_error_trampoline_super<'a, R: ObjcInstance>(receiver: *mut c_void, sel: Sel, pool: &'a ActiveAutoreleasePool, class: *const AnyClass, ($($identifier,)*): Self) -> Result<*const R, AutoreleasedCell<'a, NSError>> {
            let objc_super = ObjcSuper {
                   receiver: receiver,
                   class: class
               };
               let impcast = objc_msgSendSuper2 as unsafe extern fn();
               let mut error: *const NSError = std::ptr::null();
               let imp: unsafe extern fn(*const ObjcSuper, Sel, $( $type, )* &mut *const NSError) -> *const R  = std::mem::transmute(impcast);
               let ptr = imp(&objc_super,sel, $($identifier,)* &mut error );
               if ptr != std::ptr::null_mut() {
                   Ok(ptr)
               }
               else {
                   //I'm pretty sure it's street-legal to assume this
                   //although if it's not, don't sue me
                   Err(NSError::assume_nonnil(error).assume_autoreleased(pool))
               }
            }

        }

    );
}

//4 arguments should be enough for everybody
arguments_impl!();
arguments_impl!(a: A);
arguments_impl!(a: A, b: B);
arguments_impl!(a: A, b: B, c: C);
arguments_impl!(a: A, b: B, c: C, d: D);
arguments_impl!(a: A, b: B, c: C, d: D, e: E);
arguments_impl!(a: A, b: B, c: C, d: D, e: E, f: F);
arguments_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G);
arguments_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H);
arguments_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I);
arguments_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J); //10
arguments_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K); //11
arguments_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L); //12


#[test]
fn perform_super() {
    use objr::bindings::*;

    //We need an arbitrary subclass for this test
    objc_class! {
        pub struct NSNull {
            @class(NSNull)
        }
    }
    let pool = unsafe{ AutoreleasePool::new() };

    let o = NSNull::class().alloc_init(&pool);

    let args = ();
    //perform "super" description
    let d: *const NSString = unsafe{ <()>::invoke_super(&o as &NSNull as *const NSNull as *mut NSNull as *mut c_void, Sel::description(), &pool, NSNull::class().as_anyclass(), args) };
    let g: &NSString = unsafe{ &*d};
    let super_description = g.to_str(&pool);
    assert!(super_description.starts_with("<NSNull:"));

}

#[test] fn arguable() {
    let f = objc_nsstring!("example");
    let borrowed: &NSString = &f;
    unsafe{borrowed.assume_nonmut_perform()};

    let ptr: *const NSString = &*f;
    unsafe { ptr.assume_nonmut_perform() };

    let option: Option<&NSString> = Some(f);
    let as_ptr = option.as_ptr();
    unsafe { as_ptr.assume_nonmut_perform() };

}