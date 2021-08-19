

///! Provides a Class type that is similar to objc `AnyClass`.
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
/// The actual class type is erased.  This is generally
/// used in cases where we are bringing up an ObjcType (which would require bringing up the class first).
/// Any use of this type is likely unsafe.
#[derive(Debug)]
#[repr(transparent)]
pub struct AnyClass(c_void);


/**
Declares a static (compile-time) class reference, creates a trait to return the reference
  and implements the trait on [AnyClass].  This allows use of `let a: AnyClass = AnyClass::NSObject();` to get
  a reference to the class, where the implementation of `fn NSObject() -> AnyClass` does not need
  to lookup in the objc runtime.

This is a low-level macro, it lacks typesafety and is complex to use correctly.
For normal situations, consider [objc_class!()] instead.

This macro is often used together with the [procmacro::objc_implement_class!()].  [procmacro::objc_implement_class!()]
uses the trait declared by this macro, to implement the trait [ObjcClass] on some type.

The similar code to use of a foreign class by an ObjC compiler.  That is,
it allocates space in the symbol table for the class which will be fixedup at load time by `dyld`.

Once declared, the class can be used with `AnyClass::NSExample()`.  Note that `AnyClass` is not type-bound,
and in fact we don't rely on marker types at all. Instead, marker types rely on this.

This macro is the 'class version' of [objc_selector_group!], which does something similar for objc selectors.

# Example
```
use objr::bindings::*;
objc_any_class_trait!(
    //will declare a trait with this identifier.
    //In general you want this trait to be public and exported from the crate, see the group_name section.
    pub trait NSObjectClassTrait {
        @class(NSObject)
    }
    //This implementation will be auto-supplied, we include it just to make clear that
    //it will be implemented.
    impl NSObjectClassTrait for AnyClass {}
);


```
*/
#[macro_export]
macro_rules! objc_any_class_trait {
    (
        $(#[$attribute:meta])*
        $pub:vis trait $trait:ident {
            @class($class:ident)
        }
        impl $trait2:ident for AnyClass {}
    ) => (
        $pub trait $trait {
                ::objr::bindings::_objc_class_decl!{$class}
        }
        impl $trait for ::objr::bindings::AnyClass {
                ::objr::bindings::_objc_class_impl!{$class}
        }
    )
}

///Indicates that the given objr instance is also an objr class.
///
/// In particular, this rules out the possibility it is a protocol.
pub trait ObjcClass: ObjcInstance + Sized {
    fn class() -> &'static ClassMarker<Self>;
}


///Typed pointer to ObjC Class.  Analogous to `*const T`, but points to the class, not the instance.
///
/// Used to call "class methods" like `[alloc]`.
///
/// Example:
/// ```
/// use ::objr::bindings::*;
/// objc_class!{
/// struct NSString;
///     trait NSStringTrait {
///         @class(NSString)
///     }
///     impl NSStringTrait for AnyClass{}
/// }
/// let pool = AutoreleasePool::new();
/// let s: StrongCell<NSObject> = NSObject::class().alloc_init(&pool);
///
/// ```
#[repr(transparent)]
#[derive(Debug)]
pub struct ClassMarker<T: ObjcClass>(c_void, PhantomData<T>);

impl<T: ObjcClass> PerformablePointer for ClassMarker<T> {

}

impl<T: ObjcClass> PartialEq for ClassMarker<T> {
    fn eq(&self, other: &Self) -> bool {
        //pointer equality
        let s = self as *const Self;
        let o = other as *const Self;
        s == o
    }
}

impl<T: ObjcClass> ClassMarker<T> {
    ///Dynamically creates a Class from some string by querying the ObjC runtime.  Note that in most cases, [ClassMarker::from_anyclass] in combination
    /// with [objc_class!] macro is a faster implementation because it uses compile-time knowledge.
    pub unsafe fn from_str(cstr: &CStr) -> &'static Self {
        let dynamic_class = objc_lookUpClass(cstr.as_ptr());
        &*(dynamic_class as *const Self)
    }
    ///Converts the receiver to an anyclass
    pub fn as_anyclass(&self) -> &'static AnyClass {
        unsafe{ &*(self as *const _ as *const AnyClass) }
    }
}


impl<T: ObjcClass> ClassMarker<T> {
    ///`[[Class alloc] init]`
    ///
    pub fn alloc_init(&self, pool: &ActiveAutoreleasePool) -> StrongCell<T> {
        unsafe {
            //todo: optimize with objc_alloc_init
            let mut cell = self.alloc(pool);
            T::init(&mut cell, pool);
            let immutable = cell as *const T;
            T::assuming_nonnil(immutable).assuming_retained()
        }
    }
    ///`[Class alloc]`
    ///
    /// # Safety
    /// Unsafe becuase the underlying memory is uninitialized after this call
    pub unsafe fn alloc(&self, pool: &ActiveAutoreleasePool) -> *mut T {
        Self::perform(self as *const ClassMarker<T> as *mut _, Sel::alloc(), pool, ()) as *const T as *mut T
    }

    ///See [ObjcInstanceBehavior::assuming_nonmut_perform()]
    unsafe fn assuming_nonmut_perform(&self) -> *mut Self {
        self as *const Self as *mut Self
    }
}

impl<T: ObjcClass> std::fmt::Display for ClassMarker<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let r = unsafe {
            let pool = ActiveAutoreleasePool::assuming_autoreleasepool() ;
            let description: *const NSString = Self::perform_autorelease_to_retain(self.assuming_nonmut_perform(), Sel::description(), &pool,());
            NSString::assuming_nonnil(description).assuming_retained()
        };
        f.write_fmt(format_args!("{}",r))
    }
}




///This declares an instance type which is also a class.  See [objc_instance!] for a version which is not a class.
/// ```
/// use objr::objc_class;
/// objc_class! {
///     pub struct Example;
///     //Create an anyclass trait.  It's recommended that this be public API, see the documentation
///     pub trait ExampleAnyClassTrait {
///         @class(NSObject)
///     }
///     impl ExampleAnyClassTrait for AnyClass {} //implementation will be auto-supplied
/// }
/// ```
#[macro_export]
macro_rules! objc_class  {
    (
        $(#[$attribute:meta])*
        $pub:vis
        struct $objctype:ident;

        $(#[$traitattribute:meta])*
        $traitpub:vis
        trait $traitname:ident {
            @class($objcname:ident)
        }
        impl $trait2:ident for AnyClass {}
    ) => {
        ::objr::bindings::objc_instance! {
            $(#[$attribute])*
            $pub struct $objctype;
        }

        ::objr::bindings::objc_any_class_trait! {
            $(#[$traitattribute])*
            $traitpub trait $traitname {
                @class($objcname)
            }
            impl $trait2 for AnyClass {}
        }

        ::objr::bindings::objc_implement_class!{$objctype,$objcname}

    };
}


#[test]
fn alloc_ns_object() {
    use std::ffi::CString;
    let class = unsafe { ClassMarker::<NSObject>::from_str(CString::new("NSObject").unwrap().as_c_str() ) };
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


