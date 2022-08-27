//! objr procmacro library
//! Implements a variety of helper macros for the main crate.
//!
mod misc;
mod selectors;
mod classes;
mod instances;
mod flatten;
mod strings;
mod export_name;
mod declarations;

use proc_macro::{TokenStream, TokenTree};
use misc::{error, parse_literal_string,parse_ident};
use crate::misc::ParsedLiteral;

///```
/// # extern crate self as objr;
/// # fn main () { }
/// # use procmacro::_objc_selector_decl;
/// # mod bindings { pub struct Sel; }
/// trait Example {
///    _objc_selector_decl!{"selector"}
/// }
///
/// ```
///
/// becomes
/// ```
/// # struct Sel;
/// trait Example {
///     unsafe fn selector() -> Sel;
/// }
/// ```
#[proc_macro]
#[doc(hidden)]
pub fn _objc_selector_decl(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    use crate::selectors::{sel_to_rust_name, make_fn_partial};
    let selector = match parse_literal_string(&mut iter) {
        Ok(s) => s,
        Err(e) => return error(&format!("Expected selector, but {}",e))
    };
    let rust_name = sel_to_rust_name(&selector.unwrap_literal());
    let fn_decl = make_fn_partial(&rust_name) + ";";
    match iter.next() {
        None => (),
        Some(other) => {
            return error(&format!("Unexpected token {}",other));
        }
    }
    fn_decl.parse().unwrap()
}

///
/// ```
/// use procmacro::_objc_selector_impl;
/// # extern crate self as objr;
/// # fn main() { }
/// # mod bindings {
/// #    pub struct Sel{ pub ptr: *const std::ffi::c_void }///
/// #    impl Sel { pub unsafe fn from_ptr(ptr: *const std::ffi::c_void) -> Sel {todo!()}}
/// # }
/// #
/// trait ExampleT{ unsafe fn selector() -> ::objr::bindings::Sel; }
/// struct ExampleS;
/// impl ExampleT for ExampleS {
///     _objc_selector_impl!{"selector"}
/// }
///
/// ```
/// becomes
/// ```
/// # extern crate self as objr;
/// # fn main() { }
/// # mod bindings {
/// #    pub struct Sel;
/// # }
/// trait ExampleT{ unsafe fn selector() -> ::objr::bindings::Sel; }
/// struct ExampleS;
/// impl ExampleT for ExampleS {
///    unsafe fn selector() -> ::objr::bindings::Sel { /* static magic! */  todo!() }
/// }
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn _objc_selector_impl(stream: TokenStream) -> TokenStream {
    use selectors::{sel_to_rust_name,make_fn_partial,sel_expression};
    let mut iter = stream.into_iter();
    let selector = match parse_literal_string(&mut iter) {
        Ok(s) => s,
        Err(e) => return error(&format!("Expected selector literal, but {}",e))
    }.unwrap_literal();
    let rust_name = sel_to_rust_name(&selector);
    let mut decl = make_fn_partial(&rust_name);
    decl += &sel_expression(&selector);

    //check for extra tokens
    match iter.next() {
        None => (),
        Some(other) => {
            return error(&format!("Unexpected token {}",other));
        }
    }
    decl.parse().unwrap()
}

///Derive macro for ObjcInstance.
/// Requires the struct to be of tuple-type and have c_void
#[proc_macro_derive(ObjcInstance)]
pub fn derive_objc_instance(stream: TokenStream) -> TokenStream {
    //we're looking for something like `struct Foo`
    let mut parse_ident = false;
    let mut parsed_name = None;
    let mut item_help = None;

    //Do a flat parse, groups are dumb
    use flatten::{FlatIterator,FlatTree};
    for item in FlatIterator::new(stream.into_iter()) {
        match &item {
            FlatTree::Ident(i) if !parse_ident && i.to_string() == "struct" => {
                parse_ident = true; //about to see the type name
            }
            FlatTree::Ident(i) if parse_ident =>  {
                parsed_name = Some(i.to_string());
                break;
            }
            _ => ()
        }
        item_help = Some(item);
    }
    if parsed_name.is_none() {
        return error(&format!("Looking for `struct Identifier` near {:?}",item_help))
    }
    instances::instance_impl(&parsed_name.unwrap()).parse().unwrap()
}

///Provides an implementation of ObjcClass, based on an `objc_any_class!()` trait being in scope.
/// ```
/// # fn main() {} //https://stackoverflow.com/questions/67443775/combining-doctests-and-extern-crate/67452255#67452255
/// # extern crate self as objr; //pretend we're objr crate
/// # pub mod bindings { //shim objr objects
/// #   use std::marker::PhantomData;
/// #   pub struct Class<T: ?Sized>(core::ffi::c_void, PhantomData<T>);
/// #   pub trait ObjcClass { fn class() -> &'static Class<Self>; }
/// # }
/// use procmacro::__objc_implement_class;
/// struct RustIdentifier(core::ffi::c_void);
/// trait InScopeAutoTrait {
///     fn new() -> &'static objr::bindings::Class<RustIdentifier>;
/// }
/// impl InScopeAutoTrait for objr::bindings::Class<RustIdentifier> {
///      fn new() -> &'static objr::bindings::Class<RustIdentifier> { todo!() }
/// }
/// __objc_implement_class!{RustIdentifier}
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __objc_implement_class(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let rust_identifier = match parse_ident(&mut iter) {
        Ok(i)=> i,
        Err(err) => { return error(&format!("Expected rust identifier {:?}",err))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };
    let objc_identifier = match parse_ident(&mut iter) {
        Ok(ident) => ident,
        Err(e) => { return error(&format!("Expected objc identifier, got {}",e))}
    };
    match iter.next() {
        None => (),
        Some(e) => { return error(&format!("Expected end of macro invocation, got {:?}",e))}
    };

    let result = classes::implement_class(&rust_identifier, &objc_identifier);
    //error(&result)
    result.parse().unwrap()
}

/// Creates a compile-time NSString expression for a given literal.
///
/// Escape sequences are not currently supported and may not compile; please file a bug.
/// The expression will be of type `&'static NSString`.
/// ```
/// # extern crate self as objr;
/// # mod foundation {
/// #    pub struct NSString;
/// #
/// # }
/// # mod bindings {
/// #    use core::ffi::c_void;
/// # }
/// use procmacro::objc_nsstring;
/// # fn main() {
/// let nsstring: &'static foundation::NSString = objc_nsstring!("My test string");
/// # }
///
///
/// ```
#[proc_macro]
pub fn objc_nsstring(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let literal = match parse_literal_string(&mut iter) {
        Ok(literal) => literal,
        Err(str) => {return error(&format!("Expected a literal {}",str)) }
    }.unwrap_literal();
    let extra = iter.next();
    if extra.is_some() {
        return error(&format!("Expected end of macro near {:?}",extra.unwrap()));
    }
    strings::static_string(&literal).parse().unwrap()
}

/// Declares a static bytestring with 0 appended, with the given link_section.
///
/// It's quite difficult to concat attributes in Rust due to limitations on emitting non-items.  I can't even get munchers to inject an attribute on a macro (that expands to an item).  This is a one-shot macro that does everything for you.
/// ```
/// use procmacro::__static_asciiz;
/// __static_asciiz!("__DATA,valid_section",IDENT,"ascii");
/// ```
/// Should expand to something like
/// ```
/// #[link_section="__DATA,valid_section"]
/// static IDENT: [u8; 6] = *b"ascii\0";
/// ```
/// # Notes:
/// * the "ascii" argument may be an ident instead of a string literal
#[doc(hidden)]
#[proc_macro]
pub fn __static_asciiz(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let link_section = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        other => {return error(&format!("Expected link section literal, got {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let ident = match parse_ident(&mut iter) {
        Ok(ident) => ident,
        Err(e) => { return error(&format!("Expected identifier, got {}",e))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };
    let ascii = match misc::parse_ident_or_literal(&mut iter) {
        Ok(l) => {l}
        Err(e) => { return error(&format!("Expected literal or ident for ascii, {}",e)); }
    };

    match iter.next() {
        None => (),
        Some(e) => { return error(&format!("Expected end of macro invocation, got {:?}",e))}
    };
    export_name::export_ascii(&link_section, &ident, &ascii).parse().unwrap()

}

/// Declares a static bytestring with 0 appended, with the given link_section. Variant of [__static_asciiz] that concatenates the ident from 2 parts.
///
/// It's quite difficult to concat attributes in Rust due to limitations on emitting non-items.  I can't even get munchers to inject an attribute on a macro (that expands to an item).  This is a one-shot macro that does everything for you.
/// ```
/// use procmacro::__static_asciiz_ident2;
/// __static_asciiz_ident2!("__DATA,valid_section","IDENT_1",IDENT_2,"ascii");
/// ```
/// Should expand to something like
/// ```
/// #[link_section="__DATA,valid_section"]
/// static IDENT_1IDENT_2: [u8; 6] = *b"ascii\0";
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __static_asciiz_ident2(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let link_section = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        other => {return error(&format!("Expected link section literal, got {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let ident_1 = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
       o => { return error(&format!("Expected identifier prefix (literal), got {:?}",o))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let ident_2 = match parse_ident(&mut iter) {
        Ok(l) => {l}
        o => { return error(&format!("Expected identifier suffix (ident), got {:?}",o))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };


    let ascii = match misc::parse_ident_or_literal(&mut iter) {
        Ok(l) => {l}
        Err(e) => { return error(&format!("Expected literal or ident for ascii, {}",e)); }
    };

    match iter.next() {
        None => (),
        Some(e) => { return error(&format!("Expected end of macro invocation, got {:?}",e))}
    };


    export_name::export_ascii(&link_section, &(ident_1 + &ident_2), &ascii).parse().unwrap()
}

/// Declares a static bytestring with 0 appended, by parsing an objc declaration into a selector name. Variant of [__static_asciiz] that concatenates the ident from 2 parts and parses objc declarations.
///
/// It's quite difficult to concat attributes in Rust due to limitations on emitting non-items.  I can't even get munchers to inject an attribute on a macro (that expands to an item).  This is a one-shot macro that does everything for you.
/// ```
/// use procmacro::__static_asciiz_ident_as_selector;
/// __static_asciiz_ident_as_selector!("__DATA,valid_section","IDENT_1",IDENT_2,"-(void) example");
/// ```
/// Should expand to something like
/// ```
/// #[link_section="__DATA,valid_section"]
/// static IDENT_1IDENT_2: [u8; 8] = *b"example\0";
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __static_asciiz_ident_as_selector(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let link_section = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        other => {return error(&format!("Expected link section literal, got {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let ident_1 = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        o => { return error(&format!("Expected identifier prefix (literal), got {:?}",o))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let ident_2 = match parse_ident(&mut iter) {
        Ok(l) => {l}
        o => { return error(&format!("Expected identifier suffix (ident), got {:?}",o))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };


    let declaration = match misc::parse_ident_or_literal(&mut iter) {
        Ok(l) => {l}
        Err(e) => { return error(&format!("Expected literal or ident for ascii, {}",e)); }
    };
    match iter.next() {
        None => (),
        Some(e) => { return error(&format!("Expected end of macro invocation, got {:?}",e))}
    };

    let selector = declarations::parse_to_selector(&declaration);
    if selector.is_err() {
        return error(&selector.err().unwrap());
    }

    export_name::export_ascii(&link_section,  &(ident_1 + &ident_2), &selector.unwrap()).parse().unwrap()
}

/// Declares a static bytestring with 0 appended, by parsing an objc declaration into a type encoding. Variant of [__static_asciiz] that concatenates the ident from 2 parts and parses objc declarations.
///
/// It's quite difficult to concat attributes in Rust due to limitations on emitting non-items.  I can't even get munchers to inject an attribute on a macro (that expands to an item).  This is a one-shot macro that does everything for you.
/// ```
/// use procmacro::__static_asciiz_ident_as_type_encoding;
/// __static_asciiz_ident_as_type_encoding!("__DATA,valid_section","IDENT_1",IDENT_2,"-(void) example");
/// ```
/// Should expand to something like
/// ```
/// #[link_section="__DATA,valid_section"]
/// static IDENT_1IDENT_2: [u8; 7] = *b"v20@0:8";
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __static_asciiz_ident_as_type_encoding(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let link_section = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        other => {return error(&format!("Expected link section literal, got {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };
    let ident_1 = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        o => { return error(&format!("Expected identifier prefix (literal), got {:?}",o))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let ident_2 = match parse_ident(&mut iter) {
        Ok(l) => {l}
        o => { return error(&format!("Expected identifier suffix (ident), got {:?}",o))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };


    let declaration = match misc::parse_ident_or_literal(&mut iter) {
        Ok(l) => {l}
        Err(e) => { return error(&format!("Expected literal or ident for ascii, {}",e)); }
    };
    match iter.next() {
        None => (),
        Some(e) => { return error(&format!("Expected end of macro invocation, got {:?}",e))}
    };

    let type_encoding = declarations::parse_to_type_encoding(&declaration);
    if type_encoding.is_err() {
        return error(&type_encoding.err().unwrap());
    }
    export_name::export_ascii(&link_section, &(ident_1 + &ident_2), &type_encoding.unwrap()).parse().unwrap()
}

///Declares a static expression with `link_name` and `link_section` directives.
///
/// It's quite difficult to concat attributes in Rust due to limitations on emitting non-items.  I can't even get munchers to inject an attribute on a macro (that expands to an item).  This is a one-shot macro that does everything for you.
///
/// ```
/// use procmacro::__static_expr;
/// __static_expr!("__DATA,valid_section","EXPORT_NAME_1",EXPORT_NAME_2,static EXAMPLE: bool = false;);
/// ```
/// should expand to
/// ```
/// #[link_section="__DATA,valid_section"]
/// #[export_name="EXPORT_NAME_1EXPORT_NAME_2"]
/// static EXAMPLE: bool = false;
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __static_expr(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let link_section = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        other => {return error(&format!("Expected link section literal, got {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let export_name_1 = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        other => {return error(&format!("Expected export name literal (prefix), got {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let export_name_2 = match misc::parse_ident(&mut iter) {
        Ok(i) => {i}
        other => {return error(&format!("Expected export name (suffix) ident/literal, {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };
    let attrs = export_name::export_name_attrs(&link_section, &export_name_1, &export_name_2);
    let mut attr_stream: TokenStream = attrs.parse().unwrap();

    attr_stream.extend(iter);
    attr_stream
}

///A variant of `__static_expr` with 3 parts of the `export_name`
/// ```
/// use procmacro::__static_expr3;
/// __static_expr3!("__DATA,valid_section","EXPORT_NAME_1",EXPORT_NAME_2,"EXPORT_NAME_3",static EXAMPLE: bool = false;);
/// ```
/// should expand to
/// ```
/// #[link_section="__DATA,valid_section"]
/// #[export_name="EXPORT_NAME_1EXPORT_NAME_2EXPORT_NAME_3"]
/// static EXAMPLE: bool = false;
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __static_expr3(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let link_section = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        other => {return error(&format!("Expected link section literal, got {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let export_name_1 = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        other => {return error(&format!("Expected export name literal (prefix), got {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let export_name_2 = match misc::parse_ident(&mut iter) {
        Ok(i) => {i}
        other => {return error(&format!("Expected export name (suffix) ident/literal, {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let export_name_3 = match misc::parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(i)) => {i}
        other => {return error(&format!("Expected export name (suffix) ident/literal, {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };
    let attrs = export_name::export_name_attrs3(&link_section, &export_name_1, &export_name_2, &export_name_3);
    let mut attr_stream: TokenStream = attrs.parse().unwrap();

    attr_stream.extend(iter);
    attr_stream
}
///
/// Declares an external item.
/// ```
/// use procmacro::__static_extern;
/// extern {
///     __static_extern!("LINK_1",LINK_2, static STATIC: u32;);
/// }
/// ```
/// Expands to
/// ```
/// extern {
///     #[link_name="LINK_1LINK_2"]
///     static STATIC: u32;
/// }
/// ```
///
#[proc_macro]
pub fn __static_extern(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let link_1 = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(s)) => s,
        other => { return error(&format!("Expected link_name (prefix) literal, {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };
    let link_2 = match parse_ident(&mut iter) {
        Ok(s) => s,
        other => { return error(&format!("Expected link_name (prefix) literal, {:?}",other))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };
    let initial_str = format!(r#"
    #[link_name="{LINK_1}{LINK_2}"]
    "#, LINK_1 = link_1, LINK_2 = link_2);
    let mut attr_stream: TokenStream = initial_str.parse().unwrap();
    attr_stream.extend(iter);
    attr_stream
}

///This counts the inputs by counting the number of commas.
///
/// ```
/// use procmacro::__count;
/// let ex = __count!(a,b,c);
/// assert_eq!(ex,3);
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __count(stream: TokenStream) -> TokenStream {
    let mut count = 1;
    for item in stream {
        match item {
            TokenTree::Punct(p) if p == ',' => {count += 1},
            _ => {}
        }
    }
    count.to_string().parse().unwrap()
}

///Concatenates 2 idents into a single ident.  Mostly useful for working around macro hygeine.
///Note that this only works in a legal position, like expression position.
///
/// ```
/// use procmacro::__concat_idents;
/// let myident = 2;
/// assert_eq!(__concat_idents!("my",ident),2);
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __concat_idents(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let item1 = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        o => { return error(&format!("Expected first ident part, {:?}",o))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let item2 = match parse_ident(&mut iter) {
        Ok(i) => i,
        Err(e) => { return error(&format!("Expected second ident part, {}",e))}
    };
    return format!("{ITEM1}{ITEM2}",ITEM1=item1,ITEM2=item2).parse().unwrap()
}

#[doc(hidden)]
#[proc_macro]
pub fn __concat_3_idents(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let item1 = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        o => { return error(&format!("Expected first ident part, {:?}",o))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let item2 = match parse_ident(&mut iter) {
        Ok(i) => i,
        Err(e) => { return error(&format!("Expected second ident part, {}",e))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };
    let item3 = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        o => { return error(&format!("Expected third ident part, {:?}",o))}
    };
    return format!("{ITEM1}{ITEM2}{item3}",ITEM1=item1,ITEM2=item2).parse().unwrap()
}

///Concatenates two modules into a module declaraton.
///
/// ```
/// use procmacro::__mod;
/// __mod!(id1,id2,{
///     const example: u8 = 0;
/// });
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __mod(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let item1 = match parse_ident(&mut iter) {
        Ok(l) => {l}
        o => { return error(&format!("Expected first ident part, {:?}",o))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let item2 = match parse_ident(&mut iter) {
        Ok(i) => i,
        Err(e) => { return error(&format!("Expected second ident part, {}",e))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };
    let group = match iter.next() {
        Some(TokenTree::Group(g)) => {
            g.to_string()
        },
        o => { return error(&format!("Expected block, got {:?}",o))}
    };
    let s = format!("mod {ID1}{ID2} {BLOCK}",ID1=item1, ID2=item2,BLOCK=group);
    // return error(&s);
    s.parse().unwrap()
}

///Concatenates two ids into a use declaration
/// ```
/// mod AB {
///     pub const C:u8 = 0;
/// }
/// use procmacro::__use;
/// __use!(A,B,C);
/// mod DE {
///     pub const F:u8 = 0;
/// }
/// __use!(pub D,E,F);
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __use(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let mut item1 = match parse_ident(&mut iter) {
        Ok(l) => {l}
        _ => {
            //In case the what's here is something like $pub, but empty, o will be something like an empty group.
            //In that case, look ahead at the next token
            match parse_ident(&mut iter) {
                Ok(l) => l,
                o=> return error(&format!("Expected first ident part, {:?}",o))
            }

        }
    };
    let is_pub;
    if item1.to_string() == "pub" {
        is_pub = true;
        //parse again
        item1 = match parse_ident(&mut iter) {
            Ok(l) => {l}
            o => { return error(&format!("Expected first ident part, {:?}",o))}
        };
    }
    else {
        is_pub = false;
    }
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let item2 = match parse_ident(&mut iter) {
        Ok(i) => i,
        Err(e) => { return error(&format!("Expected second ident part, {}",e))}
    };
    match iter.next() {
        Some(TokenTree::Punct(p)) if p == ',' => (),
        o => { return error(&format!("Expected comma, got {:?}",o))}
    };

    let item3 = match parse_ident(&mut iter) {
        Ok(i) => i,
        Err(e) => { return error(&format!("Expected second ident part, {}",e))}
    };
    match iter.next() {
        None => {}
        other => { return error(&format!("Expected end of macro invocation, got {:?}",other));}
    }
    format!("{PUB} use {ID1}{ID2}::{ID3};", PUB=if is_pub { "pub"} else {""}, ID1=item1, ID2=item2, ID3=item3).parse().unwrap()
}

///Parses a literal like `"-(void) foo:(int) bar"` into a literal `"foo:"`
/// ```
/// use procmacro::__parse_declaration_to_sel;
/// __parse_declaration_to_sel!("-(void) foo:(int) bar");
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __parse_declaration_to_sel(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    let expr = match parse_literal_string(&mut iter) {
        Ok(ParsedLiteral::Literal(l)) => {l}
        o => {return error(&format!("Unexpected {:?}",o))}
    };
    let selector = declarations::parse_to_selector(&expr);
    if selector.is_err() {
        return error(&selector.err().unwrap());
    }
    let fmt = format!(r#""{}""#,selector.unwrap());
    fmt.parse().unwrap()
}