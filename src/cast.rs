/*! Cast behavior. */

/**
Allows casting from one type to another.

Use this macro to declare a safe conversion from one type to another.  This is useful for things like casting a concrete type to
a protocol type, or casting a type to a superclass type.

Arguments:
* `from` - The type to cast from.
* `to` - The type to cast to.
* `cast` - The name of the cast function to declare.  Typical values are `as_totype`.
* `cast_mut` - The name of the cast function to declare.  Typical values are `as_totype_mut`.

# Example

```
use objr::bindings::*;
//let's declare our own NSString type.  We need to do this for this example
//as objc_cast only works on types that are locally declared.
objc_class! {
    struct MyNSString {
        @class(NSString)
    }
}
objc_instance! {
    struct CFString;
}
objc_cast!(MyNSString,CFString,as_cfstring,as_cfstring_mut);


let a: &MyNSString = unsafe{objc_nsstring!("hello").cast()};
let b: &CFString = a.as_cfstring();

let c: &CFString = a.into();
```

Also works for mutable types:

```
use objr::bindings::*;
//let's declare our own NSString type.  We need to do this for this example
//as objc_cast only works on types that are locally declared.
objc_class! {
    struct MyNSString {
        @class(NSString)
    }
}
objc_instance! {
    struct CFString;
}
objc_cast!(MyNSString,CFString,as_cfstring,as_cfstring_mut);

autoreleasepool(|pool| {
    let mut nsstring = NSString::with_str_copy("hello", pool);
    let mut_nsstring: &mut MyNSString = unsafe { nsstring.cast_mut() }; //get into our local type
    let cfstring: &mut CFString = mut_nsstring.as_cfstring_mut();
    let cfstring: &mut CFString = mut_nsstring.into();
})
```
*/
#[macro_export]
macro_rules! objc_cast {
    ($from:ty,$to:ty,$methname:ident,$methname_mut:ident) => {
        impl $from {
            pub fn $methname(&self) -> &$to {
                unsafe {
                    self.cast()
                }
            }
            pub fn $methname_mut(&mut self) -> &mut $to {
                unsafe {
                    self.cast_mut()
                }
            }
        }
        impl<'s> std::convert::From<&'s $from> for &'s $to {
            fn from(a: &'s $from) -> Self {
                a.$methname()
            }
        }
        impl<'s> std::convert::From<&'s mut $from> for &'s mut $to {
            fn from(a: &'s mut $from) -> Self {
                a.$methname_mut()
            }
        }
    };
}