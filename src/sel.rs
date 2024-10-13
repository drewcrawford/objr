//SPDX-License-Identifier: MIT OR Apache-2.0

use std::ffi::{c_void, CString};
use std::os::raw::c_char;

#[link(name="objc", kind="dylib")]
extern "C" {
    fn sel_registerName(string: *const c_char) -> *const c_void;
}

///ObjC-compatible selector.  This type is repr-transparent and can go over the wire as an arg.
#[derive(Copy,Clone,Debug)]
#[repr(transparent)]
pub struct Sel(*const c_void);
impl Sel {
    ///Dynamically creates `Sel` from a string by quering the ObjC runtime.  Note that in most cases, [crate::bindings::objc_selector_group!()] is a faster method
    /// to get selectors.
    pub fn from_str(string: &str) -> Self {
        let cstring = CString::new(string).unwrap();

        Sel(unsafe { sel_registerName(cstring.as_ptr()) })
    }
    pub unsafe fn ptr(&self) -> *const c_void {
        self.0
    }
    pub const fn from_ptr(ptr: *const c_void) -> Sel {
        Sel(ptr)
    }

}

///Primarily used by [objc_subclass!] and similar.
#[repr(transparent)]
#[doc(hidden)]
pub struct _SyncWrapper<T>(pub T);
unsafe impl<T> core::marker::Sync for _SyncWrapper<T> {}


//this magic is needed for dyld to think our program is objc and fixup our symbols
#[link_section = "__DATA,__objc_imageinfo,regular,no_dead_strip"]
#[export_name = "\x01L_OBJC_IMAGE_INFO"]
#[used]
static IMAGE_INFO: [u32; 2] = [0, 64];


///Statically declares a selector and makes it available for use.
///
/// Before the program entrypoint, dyld will identify these selectors and replace them
/// with the value known to the ObjC runtime.  This is substantially faster than `Sel::from_str()` which is a runtime behavior
/// that involves acquiring a lock.
///
/// # Example
/// ```
/// use objr::objc_selector_group;
/// use objr::bindings::*;
/// objc_selector_group!(
///         //Declare a trait.  The trait will have members for each selector.
///         trait NSObjectSelectors {
///             //each ObjC selector, in normal ObjC selector syntax
///             @selector("description")
///             @selector("respondsToSelector:")
///             @selector("init")
///         }
///         //Implement the trait on Sel.  This allows the use of `Sel::description()` etc.
///         impl NSObjectSelectors for Sel {}
///     );
/// unsafe {
///     let my_selector = Sel::description();
/// }
/// ```
#[macro_export]
macro_rules! objc_selector_group {
    (
        $(#[$attribute:meta])*
        $pub:vis trait $trait:ident {
            $(
            @selector($selector:literal))*
        }
        impl $trait2:ident for Sel {}
    ) => (
        $pub trait $trait {
            $(
                objr::bindings::_objc_selector_decl!{$selector}
            )*
        }
        impl $trait for objr::bindings::Sel {
            $(
                objr::bindings::_objc_selector_impl!{$selector}
            )*
        }
    )
}
