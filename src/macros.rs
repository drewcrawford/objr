//! Implements a variety of macros for simple objc binding declarations




///Helps generate bindings for an objc enum, as a struct with const members.
///
/// # example
///
/// ```
///# use objr::bindings::*;
///objc_enum! {
///     pub struct MTLPixelFormat<u32>;
///     impl MTLPixelFormat {
///         MTLPixelFormatInvalid = 0
///     }
/// }
///```
/// # Notes
/// This macro requires
/// * a struct with a single field
/// * implementation block
/// * value-level macros, like `API_AVAILABLE`, to be removed.  If you need to figure out a situation for old OS, do it yourself.
///   You can find and remove such lines with the regex `API_AVAILABLE\(.*\)`.
/// * Certain complex comments need to be removed, although simple block comments appear to work in my testing.
#[macro_export]
macro_rules! objc_enum {
    (
        $(#[$attribute:meta])*
        $pub:vis struct $enum:ident<$type:ty>;
        impl $ignore:ident {
            $($a:ident = $b:expr),*
        }
    ) => (
        $(#[$attribute])*
        $pub struct $enum($type);
        #[allow(non_upper_case_globals)]
        impl $enum {
           $($pub const $a: $enum = $enum($b);)*
           $pub const fn field(&self) -> $type { self.0 }
        }

    )
}

