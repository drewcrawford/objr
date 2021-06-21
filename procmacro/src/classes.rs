///Emits `unsafe fn NSExample() -> AnyClass` with no semicolon
pub fn make_fn_partial(class_name: &str) -> String {
    format!("unsafe fn {}() -> ::objr::bindings::AnyClass",class_name)
}



pub fn implement_any_class(class_name: &str, group_name: &str) -> String {
    let header = make_fn_partial(class_name);
    let body = format!(
        r#"{{
        #[inline(never)] unsafe fn merge_compilation_units() -> ::objr::bindings::AnyClass {{
        extern {{
            //this link name needs to be exactly this so the linker understands we're doing an objc class
            #[link_name = "\x01_OBJC_CLASS_$_{CLASS_NAME}"]
            static CLASS : *mut core::ffi::c_void;
        }}

            #[link_section="__DATA,__objc_classrefs,regular,no_dead_strip"]
            //in practice, seems like this can be L_Anything
            //but it needs to not conflict with multiple declarations
            #[export_name="\x01L_OBJC_CLASSLIST_REFERENCES.{GROUP_NAME}.{CLASS_NAME}"]
            static CLASS_REF: ::objr::bindings::_SyncWrapper<*mut core::ffi::c_void> = ::objr::bindings::_SyncWrapper(unsafe{{ std::mem::transmute(&CLASS) }});
            ::objr::bindings::AnyClass::from_ptr(::core::ptr::read_volatile(&CLASS_REF.0))
        }}
        merge_compilation_units()
    }}"#,CLASS_NAME=class_name,GROUP_NAME=group_name);
    let result = header + "\n" + &body;
    result
    // let safe_result = result.replace('"',"\\\"");
    // format!("compile_error!(\"{}\")",safe_result)
}

pub fn implement_class(rust_name: &str, objc_class_name: &str) -> String {
    format!(r#"
        impl ::objr::bindings::ObjcClass for {RUST_NAME} {{
            fn class() -> ::objr::bindings::ClassMarker<{RUST_NAME}> {{
                unsafe{{ ::objr::bindings::ClassMarker::from_anyclass(::objr::bindings::AnyClass::{CLASS}()) }}
            }}
        }}
    "#,CLASS=objc_class_name, RUST_NAME=rust_name)
}