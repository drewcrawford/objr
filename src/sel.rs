use std::ffi::{c_void, CString};
use std::os::raw::c_char;

#[link(name="objc", kind="dylib")]
extern "C" {
    fn sel_registerName(string: *const c_char) -> *const c_void;
}

///ObjC-compatible selector.  This type is repr-transparent and can go over the wire as an arg.
#[derive(Copy,Clone,Debug)]
#[repr(transparent)]
pub struct Sel {
    #[allow(dead_code)]
    pub ptr: *const c_void,
}
impl Sel {
    ///Dynamically creates `Sel` from a string by quering the ObjC runtime.  Note that in most cases, [objc_selector_group!()] is a faster method
    /// to get selectors.
    pub fn from_str(string: &str) -> Self {
        let cstring = CString::new(string).unwrap();

        Sel {
            ptr: unsafe { sel_registerName(cstring.as_ptr()) }
        }
    }
    pub unsafe fn ptr(&self) -> *const c_void {
        self.ptr
    }

}

///Primarily used by [objc_selector_impl!] and similar.
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
///         trait NSObjectSelectors {
///             @selector("description")
///             @selector("respondsToSelector:")
///             @selector("init")
///         }
///         impl NSObjectSelectors for Sel {}
///     );
/// unsafe {
///     let my_selector = Sel::description();
/// }
/// ```
/// # Detailed usage
/// The first part of the macro declares a `trait`.
/// Inside the `trait` is a `group_name`.  See the [#group_name] section below.
/// Then comes a list of `@selector("name")` entries.
/// Finally, an `impl` block for `Sel`.  The actual implementation will be supplied automatically.
/// Note that you must have `objr::bindings::sel_macroscope` in scope.  In general I recommend you import `objr::bindings::*`; if you are doing bindings work.
///
/// # group_name
/// ObjC symbols in Rust have conflicting requirements:
//
// 1.  They must be public, since they must be fixedup by dyld at runtime
// 2.  They might be private, since using particular ObjC APIs may be an internal implementation detail of some module
// 3.  Each symbol should appear exactly once, to produce a small binary and get fast load performance
// 4.  Each symbol can appear more than once, since different modules might use the same APIs unbeknownst to each other.
///
/// In the general case, these problems are unsolveable.  However, presumably you have a program to compile so you
/// need *some* solution.  The `group_name` is the API that lets you explain a particular solution to the compiler
/// so as to build your program.
///
/// All ObjC symbols are mangled only with `group_name`, as opposed to Rust mangling normally.  This means that
/// the same symbol with the same `group_name` will conflict if declared more than once.  This may not show up in your
/// testing, but only when some larger program is linked together will the conflict be discovered.
///
/// A simple rule is to choose `group_name` with a unique value like the crate name.  Then your problems can only
/// come from within your crate, since presumably other crates will use another `group_name`.
///
/// You should also consider marking the magic trait public and exporting it from the crate, even if it does not otherwise
/// seem like a public API.  This is because, as discussed, at the symbol level it is public anyway and this
/// gives users the option of reusing that symbol
/// for their own purposes instead of being forced to add a second one.  Of course doing this has API implications
/// so you may want to consider that angle.
///
/// Finally, you may convince yourself that duplication is actually impossible.  This is trivially true inside a crate
/// (duplicate symbols will fail to link), but maybe you have some other strategy such as you maintain two libraries
/// and know they don't have conflicts.  If so, you can share `group_name` safely between them, but keep in mind
/// if there is some duplication they will fail to link.
///
/// In summary:
/// 1.  Consider using something unique like the crate name for `group_name`
/// 2.  Consider exporting the trait as public API, even if you wouldn't otherwise.
///
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
        impl $trait for Sel {
            $(
                objr::bindings::_objc_selector_impl!{$selector}
            )*
        }
    )
}
