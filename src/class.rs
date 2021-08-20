///! Implementation of ObjC classes.  Classes are distinct from instances (which could be, for example, protocols).
use std::ffi::{c_void, CStr};
use super::performselector::PerformablePointer;
use super::bindings::*;
use std::os::raw::c_char;
use core::marker::PhantomData;
use std::fmt::Formatter;

#[link(name="objc", kind="dylib")]
extern "C" {
    fn objc_lookUpClass(name: * const c_char) -> *mut c_void;
}

///Untyped pointer to ObjC class.
///
/// The actual class type is erased.  Any use of this type is likely unsafe.
#[derive(Debug)]
#[repr(transparent)]
pub struct AnyClass(c_void);

///A trait for Rust types that map to ObjC classes.
///
/// This is similar to [ObjcInstance] (and requires it) but imposes additional class requirements.
///
/// In particular, this rules out the possibility it is a protocol.
///
///
/// # Stability
/// It is not stable API to impelment this trait directly.  Instead use the [objc_class!] macro.
///
/// # Safety
/// This is safe because the linker checks that this is a valid class
pub trait ObjcClass: ObjcInstance + Sized {
    fn class() -> &'static Class<Self>;
}



///Typed pointer to ObjC Class.  Analogous to `*const T`, but points to the class, not the instance.
///
/// Used to call "class methods" like `[alloc]`.
///
/// To create this type, it's recommended to use `Class::new()`.  For more information, see [objc_class!].
#[repr(transparent)]
#[derive(Debug)]
pub struct Class<T: ObjcClass>(c_void, PhantomData<T>);

///Classes can use performSelector
unsafe impl<T: ObjcClass> PerformablePointer for Class<T> {}

impl<T: ObjcClass> PartialEq for Class<T> {
    fn eq(&self, other: &Self) -> bool {
        //pointer equality
        let s = self as *const Self;
        let o = other as *const Self;
        s == o
    }
}

impl<T: ObjcClass> Class<T> {
    ///Dynamically creates a Class from some string by querying the ObjC runtime.  Note that in most cases, [NSObject::class()] in combination
    /// with [objc_class!] macro is a faster implementation because it uses compile-time knowledge.
    pub unsafe fn from_str(cstr: &CStr) -> &'static Self {
        let dynamic_class = objc_lookUpClass(cstr.as_ptr());
        &*(dynamic_class as *const Self)
    }
    ///Converts to an anyclass
    pub fn as_anyclass(&self) -> &'static AnyClass {
        unsafe{ &*(self as *const _ as *const AnyClass) }
    }
}


impl<T: ObjcClass> Class<T> {
    ///`[[Class alloc] init]`
    ///
    pub fn alloc_init(&self, pool: &ActiveAutoreleasePool) -> StrongCell<T> {
        unsafe {
            //todo: optimize with objc_alloc_init
            let mut cell = self.alloc(pool);
            T::init(&mut cell, pool);
            let immutable = cell as *const T;
            T::assume_nonnil(immutable).assume_retained()
        }
    }
    ///`[Class alloc]`
    ///
    /// # Safety
    /// Unsafe because the underlying memory is uninitialized after this call
    pub unsafe fn alloc(&self, pool: &ActiveAutoreleasePool) -> *mut T {
        Self::perform(self as *const Class<T> as *mut _, Sel::alloc(), pool, ()) as *const T as *mut T
    }

    ///See [ObjcInstanceBehavior::assume_nonmut_perform()]
    pub unsafe fn assume_nonmut_perform(&self) -> *mut Self {
        self as *const Self as *mut Self
    }
}

impl<T: ObjcClass> std::fmt::Display for Class<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let r = unsafe {
            let pool = ActiveAutoreleasePool::assume_autoreleasepool() ;
            let description: *const NSString = Self::perform_autorelease_to_retain(self.assume_nonmut_perform(), Sel::description(), &pool,());
            NSString::assume_nonnil(description).assume_retained()
        };
        f.write_fmt(format_args!("{}",r))
    }
}

///This declares an instance type which is also a class.  See [objc_instance!] for a version which is not a class.
/// ```
/// use objr::bindings::*;
/// objc_class! {
///     //Declare a struct with this name, representing our objc class
///     pub struct Example {
///         @class(NSObject)
///     }
/// }
/// let pool = AutoreleasePool::new();
/// let instance = Example::class().alloc_init(&pool);
/// let class = Example::class();
/// ```
#[macro_export]
macro_rules! objc_class  {
    (
        $(#[$attribute:meta])*
        $pub:vis
        struct $objctype:ident {
            @class($objcname:ident)
        }
    ) => {
        ::objr::bindings::objc_instance! {
            $(#[$attribute])*
            $pub struct $objctype;
        }
        ::objr::bindings::__objc_implement_class!{$objctype,$objcname}
    };
}


#[test]
fn alloc_ns_object() {
    use std::ffi::CString;
    let class = unsafe { Class::<NSObject>::from_str(CString::new("NSObject").unwrap().as_c_str() ) };
    println!("{}",class);
}
#[test]
fn init_ns_object() {
    use crate::autorelease::AutoreleasePool;
    let pool = AutoreleasePool::new();
    let class =  NSObject::class();
    let class2 =  NSObject::class();
    assert_eq!(class, class2);
    let instance =  class.alloc_init(&pool);
    let description = instance.description(&pool);
    assert!(description.to_str(&pool).starts_with("<NSObject"))
}

