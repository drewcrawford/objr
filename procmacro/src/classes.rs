//! Contains the implementation for ::objr::bindings::ObjcClass.


///Emits `unsafe fn new() -> &Class<T>` with no semicolon
pub fn make_fn_partial(rust_name:&str) -> String {
    format!("fn new() -> &'static ::objr::bindings::Class<{RUST_NAME}>",RUST_NAME=rust_name)
}



pub fn implement_class_type(class_name: &str,rust_name:&str) -> String {
    let header = make_fn_partial(rust_name);
    let body = format!(
        r#"{{
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
    }}"#,CLASS_NAME=class_name,RUST_NAME=rust_name);
    let result = header + "\n" + &body;
    result
    // let safe_result = result.replace('"',"\\\"");
    // format!("compile_error!(\"{}\")",safe_result)
}

pub fn implement_class(rust_name: &str) -> String {
    format!(r#"
        impl ::objr::bindings::ObjcClass for {RUST_NAME} {{
            fn class() -> &'static ::objr::bindings::Class<{RUST_NAME}> {{
                ::objr::bindings::Class::<Self>::new()
            }}
        }}
    "#, RUST_NAME=rust_name)
}