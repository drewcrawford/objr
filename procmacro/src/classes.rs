///Emits `unsafe fn NSExample() -> AnyClass` with no semicolon
pub fn make_fn_partial(class_name: &str) -> String {
    format!("unsafe fn {}() -> &'static ::objr::bindings::AnyClass",class_name)
}



pub fn implement_any_class(class_name: &str) -> String {
    let header = make_fn_partial(class_name);
    let body = format!(
        r#"{{
        #[inline(never)] unsafe fn merge_compilation_units() -> &'static ::objr::bindings::AnyClass {{
        extern {{
            //this link name needs to be exactly this so the linker understands we're doing an objc class
            #[link_name = "\x01_OBJC_CLASS_$_{CLASS_NAME}"]
            static CLASS : *mut core::ffi::c_void;
        }}

            #[link_section="__DATA,__objc_classrefs,regular,no_dead_strip"]
            //in practice, seems like this can be L_Anything
            //but it needs to not conflict with multiple declarations
            static CLASS_REF: &'static ::objr::bindings::AnyClass = unsafe{{ std::mem::transmute(&CLASS) }};
            ::core::ptr::read_volatile(&CLASS_REF)
        }}
        merge_compilation_units()
    }}"#,CLASS_NAME=class_name);
    let result = header + "\n" + &body;
    result
    // let safe_result = result.replace('"',"\\\"");
    // format!("compile_error!(\"{}\")",safe_result)
}

pub fn implement_class(rust_name: &str, objc_class_name: &str) -> String {
    format!(r#"
        impl ::objr::bindings::ObjcClass for {RUST_NAME} {{
            fn class() -> &'static ::objr::bindings::ClassMarker<{RUST_NAME}> {{
                unsafe{{ std::mem::transmute(::objr::bindings::AnyClass::{CLASS}()) }}
            }}
        }}
    "#,CLASS=objc_class_name, RUST_NAME=rust_name)
}