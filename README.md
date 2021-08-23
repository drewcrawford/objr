# Drew's very fast objc library
This library provides low-level bindings to ObjC, which is in practice the ABI for macOS.  You might compare
this crate with [objc](https://crates.io/crates/objc), [fruity](https://docs.rs/fruity/0.2.0/fruity/), [objcrs](https://crates.io/crates/objrs)
and many others.

Distinctive features of this library include:
* Zero-cost abstractions, including the elusive "compile-time selectors" as well as many other new and exciting static technologies
* Advanced performance and codesize optimizations suitable for real-time, game, and graphical applications
* Smart pointers that integrate ObjC memory management into safe Rust abstractions
* Write ObjC subclasses directly in Rust
* Emits code similar to real ObjC compilers, for speed and future-compatibility
* Macro system for productively hand-coding bindings for new ObjC APIs
* Low level, ObjC-like APIs that you can build on to expose a Rusty interface – or not, for extra control and performance
* Minimal API coverage, leaving the question of which APIs to use and how to expose them to Rust for other crates
* Focus on modern, Apple-platform ObjC
* Free for noncommercial use

# Detailed general examples

## Using an external class

```rust
//We intend to write bindings for ObjC APIs
use objr::bindings::*;
objc_class! {
    //Rust wrapper type
    pub struct NSDate {
        //ObjC class name
        @class(NSDate)
    }
}
autoreleasepool(|pool| {
  //In this library, autoreleasepools are often arguments to ObjC-calling APIs, providing static guarantees you created one.
  //Forgetting this is a common ObjC bug.
  let date = NSDate::class().alloc_init(&pool);
  println!("{}",date); // 2021-06-21 19:03:15 +0000
})
```

Compare this with `objc_instance!` for non-class instances.

## Binding an ObjC API

```rust
use objr::bindings::*;
objc_class! {
    //Rust wrapper type
    pub struct NSDate {
        @class(NSDate)
    }
}
//Declares a group of static selectors.
objc_selector_group! {
    pub trait NSDateSelectors {
        @selector("dateByAddingTimeInterval:")
    }
    //Adds support for these selectors to our `Sel` APIs.
    impl NSDateSelectors for Sel {}
}

//Declare APIs directly on our `NSDate` wrapper type
impl NSDate {
    fn dateByAddingTimeInterval(&self, pool: &ActiveAutoreleasePool, interval: f64)
    //Although the underlying ObjC API returns a +0 unowned reference,
    //We create a binding that returns +1 retained instead.  We might do this
    //because it's the preferred pattern of our application.
    //For more details, see the documentation of [objc_instance!]
    -> StrongCell<NSDate> {
        //Use of ObjC is unsafe.  There is no runtime or dynamic checking of your work here,
        //so you must provide a safe abstraction to callers (or mark the enclosing function unsafe).
        unsafe {
            /*Convert from an autoreleased return value to a strong one.
            This uses tricks used by real ObjC compilers and is far faster than calling `retain` yourself.
            */
            let raw = Self::perform_autorelease_to_retain(
                //the objc method we are calling does not mutate the receiver
                self.assume_nonmut_perform(),
                ///Use the compile-time selector we declared above
                Sel::dateByAddingTimeInterval_(),
                ///Static checking that we have an autoreleasepool available
                 pool,
                 ///Arguments.  Note the trailing `,`.  Arguments are tuple types.
                 (interval,));
            //assume the result is nonnil
            Self::assume_nonnil(raw)
            //assume the object is +1 convention (it is, because we called perform_autorelease_to_retain above)
                .assume_retained()
        }
    }
}
autoreleasepool(|pool| {
    //In this library, autoreleasepools are often arguments to ObjC-calling APIs, providing compile-time guarantees you created one.
    //Forgetting this is a common ObjC bug.
    let date = NSDate::class().alloc_init(&pool);
    let new_date = date.dateByAddingTimeInterval(&pool, 23.5);
})
```

For more examples, see the documentation for `objc_instance!`.

# Feature index

* Statically declare selectors and classes, string literals, enums, etc. so they don't have to be looked up at runtime
* Leverage the Rust typesystem to elide `retain`/`release`/`autorelease` calls in many cases.
* Participate in runtime autorelease eliding which reduces memory overhead when calling system code
  This means that for programs that are mostly Rust, codegeneration may be significantly better even than real ObjC programs.
* Pointer packing for `Option<&NSObject>`
* Smart pointer system, with support for `StrongCell` and `AutoreleasedCell`
* Subclassing directly from Rust
* (limited) support for mutability and exclusive references in imported types

Not yet implemented, but planned or possible:

* iOS support
* Exceptions (Debug-quality API available now, see ``bindings::try_unwrap_void`)

# Design limitations

This library intends to follow normal guidelines for safe Rust.  However, calling into ObjC means there's
a giant ball of `unsafe` somewhere.

This library makes the assumption that the underlying ball of ObjC is implemented correctly.  In particular,
it avoids runtime checks that ObjC is implemented correctly, so to the extent that it isn't, you may encounter UB.

For more information, see the safety section of `objc_instance`!