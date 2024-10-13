//SPDX-License-Identifier: MIT OR Apache-2.0

//! Contains the implementation for ::objr::bindings::ObjcClass.





pub fn implement_class(rust_name: &str,class_name: &str) -> String {
    format!(r#"
        impl ::objr::bindings::ObjcClass for {RUST_NAME} {{
            fn class() -> &'static ::objr::bindings::Class<{RUST_NAME}> {{
                #[inline(never)] unsafe fn merge_compilation_units() -> &'static ::objr::bindings::Class<{RUST_NAME}> {{
        extern {{
            //this link name needs to be exactly this so the linker understands we're doing an objc class
            #[link_name = "\x01_OBJC_CLASS_$_{CLASS_NAME}"]
            static CLASS : *mut core::ffi::c_void;
        }}

            #[link_section="__DATA,__objc_classrefs,regular,no_dead_strip"]
            //in practice, seems like this can be L_Anything
            //but it needs to not conflict with multiple declarations
            static CLASS_REF: &'static ::objr::bindings::Class<{RUST_NAME}> = unsafe{{ std::mem::transmute(&CLASS) }};
            ::core::ptr::read_volatile(&CLASS_REF)
        }}
        unsafe{{ merge_compilation_units() }}
            }}
        }}
    "#, RUST_NAME=rust_name,CLASS_NAME=class_name)
}