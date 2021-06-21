

///! Provides a Class type that is similar to objc `AnyClass`.
use std::ffi::{c_void, CStr};
use super::performselector::PerformablePointer;
use super::bindings::*;
use std::os::raw::c_char;

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
pub struct AnyClass(*mut c_void);

impl AnyClass {
    ///Unsafe because we literally check nothing, not if the pointer is valid or anything
    ///Unsafe because use of this class in general is not typesafe
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Self { AnyClass(ptr)}
}

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
        //indicates the group_name, see the appropriate section
        let group_name = "example";
        @class(NSObject)
    }
    //This implementation will be auto-supplied, we include it just to make clear that
    //it will be implemented.
    impl NSObjectClassTrait for AnyClass {}
);


```
# `group_name`
See the section in [objc_selector_group!()] for the gory details, but in summary:
1.  Consider using something unique like the crate name for `group_name`
2.  Consider exporting the trait as public API, even if you wouldn't otherwise.
*/
#[macro_export]
macro_rules! objc_any_class_trait {
    (
        $(#[$attribute:meta])*
        $pub:vis trait $trait:ident {
            let group_name = $group_name:literal;
            @class($class:ident)
        }
        impl $trait2:ident for AnyClass {}
    ) => (
        $pub trait $trait {
                ::objr::bindings::_objc_class_decl!{$class}
        }
        impl $trait for ::objr::bindings::AnyClass {
                ::objr::bindings::_objc_class_impl!{$class,$group_name}
        }
    )
}

///Indicates that the given objr instance is also an objr class.
///
/// In particular, this rules out the possibility it is a protocol.
pub trait ObjcClass: ObjcInstance {
    fn class() -> ClassMarker<Self>;
}





///Typed pointer to ObjC Class.  Analogous to `Marker`, but for the "class" instead of the instance.
///
/// Used to call "class methods" like `[alloc]`.
///
/// Example:
/// ```
/// use ::objr::bindings::*;
/// objc_class!{
/// struct NSString;
///     trait NSStringTrait {
///         let group_name="example";
///         @class(NSString)
///     }
///     impl NSStringTrait for AnyClass{}
/// }
/// autoreleasepool(|pool| {
///     let s: StrongCell<NSObject> = NSObject::class().alloc_init(pool);
/// })
///
/// ```
#[repr(transparent)]
#[derive(Debug)]
pub struct ClassMarker<T: ObjcClass + ?Sized>(*mut c_void, PhantomData<T>);

impl<T: ObjcClass> ClassMarker<T> {
    ///Unsafe because we don't check the type, or that anyclass is valid at all
    pub unsafe fn from_anyclass(anyclass: AnyClass) -> Self {
        ClassMarker(anyclass.0, PhantomData::default())
    }
    ///Dynamically creates a Class from some string by querying the ObjC runtime.  Note that in most cases, [ClassMarker::from_anyclass] in combination
    /// with [objc_class!] macro is a faster implementation because it uses compile-time knowledge.
    pub unsafe fn from_str(cstr: &CStr) -> Self {
        let dynamic_class = objc_lookUpClass(cstr.as_ptr());
        ClassMarker(dynamic_class, PhantomData::default())
    }
    ///Converts the receiver to an anyclass
    pub fn as_anyclass(&self) -> AnyClass {
        AnyClass(self.0)
    }
}

impl<T: ObjcClass> ClassMarker<T> {
    ///`[[Class alloc] init]`
    ///
    pub fn alloc_init(&self, pool: &ActiveAutoreleasePool) -> StrongCell<T> {
        unsafe {
            let mut cell = self.alloc(pool);
            cell.init(pool);
            cell.assuming_retained()
        }
    }
    ///`[Class alloc]`
    ///
    /// # Safety
    /// Unsafe becuase the underlying memory is uninitialized after this call
    pub unsafe fn alloc(&self, pool: &ActiveAutoreleasePool) -> UnwrappedCell<T> {
        use super::performselector::PerformsSelectorPrivate;
        UnwrappedCell::new(self.perform_unmanaged_nonnull(Sel::alloc(), pool, ()))
    }
}

impl<T: ObjcClass> PerformablePointer for ClassMarker<T> {
    unsafe fn ptr(&self) -> *mut c_void {
        self.0
    }
}

#[cfg(test)]
use super::bindings::autoreleasepool;
use std::marker::PhantomData;

///This declares an instance type which is also a class.  See [objc_instance!] for a version which is not a class.
/// ```
/// use objr::objc_class;
/// objc_class! {
///     pub struct Example;
///     //Create an anyclass trait.  It's recommended that this be public API, see the documentation
///     pub trait ExampleAnyClassTrait {
///         //see the section on group_name
///         let group_name = "example";
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
            let group_name = $group_name:literal;
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
                let group_name = $group_name;
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
    assert!(!class.0.is_null())
}
#[test]
fn init_ns_object() {
    autoreleasepool(|pool| {
        let class =  NSObject::class();
        let class2 =  NSObject::class();
        assert!(!class.0.is_null());
        assert_eq!(class.0, class2.0);
        let instance =  class.alloc_init(pool);
        let description = instance.description(pool);
        assert!(description.to_str(pool).starts_with("<NSObject"))
    })
}


