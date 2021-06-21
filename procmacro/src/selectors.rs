//! Selector helper functions
extern crate proc_macro;

///An expression for a `Sel` with a dyld-time static
pub fn sel_expression(selector: &str, selector_group: &str) -> String {
    let symbol_name = selector.replace(":","_");
    format!(
        r#"
    {{
        #[inline(never)] unsafe fn codegen_workaround() -> ::objr::bindings::Sel {{
            #[link_section = "__TEXT,__objc_methname,cstring_literals"]
            #[export_name = "\x01L_OBJC_METH_VAR_NAME_.{selector_group}.{symbol_name}"]
            static L_OBJC_METH_VAR_NAME_: [u8; {len}] = *b"{selector}\0";

            #[link_section = "__DATA,__objc_selrefs,literal_pointers,no_dead_strip"]
            #[export_name = "\x01L_OBJC_SELECTOR_REFERENCES_.{selector_group}.{symbol_name}"]
            static L_OBJC_SELECTOR_REFERENCES_: ::objr::bindings::_SyncWrapper<*const u8> = ::objr::bindings::_SyncWrapper(&L_OBJC_METH_VAR_NAME_ as *const _);
            //don't let the optimizer look at the value we just set, since it will be fixedup by dyld
            let read_volatile = ::core::ptr::read_volatile(&L_OBJC_SELECTOR_REFERENCES_.0);
            ::objr::bindings::Sel {{
               ptr: unsafe{{ std::mem::transmute(read_volatile) }}
            }}
        }}
        codegen_workaround()
    }}"#
        ,selector_group=selector_group,selector=selector,len=selector.len() + 1,symbol_name=symbol_name)
}

///Declares a "partial" fn like `unsafe fn my_selector() -> ::objr::bindings::Sel` with no trailing `;`
pub fn make_fn_partial(fn_name: &str) -> String {
    format!("unsafe fn {fn_name}() -> ::objr::bindings::Sel",fn_name=fn_name)
}


///Finds an appropriate rust name for a given selector
pub fn sel_to_rust_name(selector: &str) -> String {
    let mut rust_build = String::new();
    let mut seen_colon_count: u8 = 0;
    for char in selector.chars() {
        match char {
            ':' => {
                //generally we replace `:` with `_` for rust
                rust_build.push('_');
                seen_colon_count+=1;
            }
            other => { rust_build.push(other);}
        }
    }
    /*In objc, we can have these selectors
    * `height` => `fn height()`
    * `height:` (with an argument) => `fn height_(arg: Type)`.  Note that in Rust we need a distinct name `height_` to avoid
       conflict with `height`
    * `height:width:` => fn height_width(arg: Type, arg2: Type)`.  No trailing underscore required here

    This selector is not legal:
    x `height:width`.  Since this is illegal, the name `height_width` is not 'reserved' for it, and can be used for `height:width:` instead.

    Shorter version, if our colon count is >1 we can remove the trailing `_`.
     */
    if seen_colon_count > 1 {
        rust_build.pop();
    }
    rust_build
}




#[test]
fn build_selector() {
    assert_eq!(sel_to_rust_name("height"), "height");
    assert_eq!(sel_to_rust_name("height:"), "height_");
    assert_eq!(sel_to_rust_name("height:width:"), "height_width");
}
