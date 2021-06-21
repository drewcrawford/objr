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
	pub struct NSDate;
	//defines a "group" for the symbol.  See documentation
	pub trait NSDateTrait {
		let group_name="example";
		//ObjC class name
		@class(NSDate)
	}
	//Add support for NSDate onto our `AnyClass` APIs.
	impl NSDateTrait for AnyClass {}
}
autoreleasepool(|pool| {
	//In this library, autoreleasepools are often arguments to ObjC-calling APIs, providing compile-time guarantees you created one.
	//Forgetting this is a common ObjC bug.
	let date = NSDate::class().alloc_init(pool);
	println!("{}",date); // 2021-06-21 19:03:15 +0000
});
```

Compare this with  `objc_instance!` for non-class instances.

## Binding an ObjC API

```rust
use objr::bindings::*;
objc_class! {
	//Rust wrapper type
	pub struct NSDate;
	//defines a "group" for the symbol.  See documentation
	pub trait NSDateTrait {
		let group_name="example";
		//ObjC class name
		@class(NSDate)
	}
	//Add support for NSDate onto our `AnyClass` APIs.
	impl NSDateTrait for AnyClass {}
}
//Declares a group of static selectors.
objc_selector_group! {
	pub trait NSDateSelectors {
		let group_name = "example";
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
	-> StrongCell<NSDate> {
		//Use of ObjC is unsafe.  There is no runtime or dynamic checking of your work here,
		//so you must provide a safe abstraction to callers (or mark the enclosing function unsafe).
		unsafe {
			//access the internal `marker` type for this NSDate instance
			self.marker()
			/*Convert from an autoreleased return value to a strong one.
			This uses tricks used by real ObjC compilers and is far faster than calling `retain` yourself.
			*/
			.perform_autorelease_to_strong_nonnull(
				///Use the compile-time selector we declared above
				Sel::dateByAddingTimeInterval_(),
				///Static checking that we have an autoreleasepool available
				 pool,
				 ///Arguments.  Note the trailing `,`
				 (interval,))
		}
	}
}

autoreleasepool(|pool| {
	//In this library, autoreleasepools are often arguments to ObjC-calling APIs, providing compile-time guarantees you created one.
	//Forgetting this is a common ObjC bug.
	let date = NSDate::class().alloc_init(pool);
	let new_date = date.dateByAddingTimeInterval(pool, 23.5);
});
```


# Feature index

(See rustdoc for links in this table)

* Statically declare selectors and classes, string literals, enums, etc. so they don't have to be looked up at runtime
	* "Groups" that help manage (unmangled) static symbols across crates and compilation units
* Leverage the Rust typesystem to elide `retain`/`release`/`autorelease` calls in many cases.
* Participate in runtime autorelease eliding which reduces memory overhead when calling system code
This means that for programs that are mostly Rust, codegeneration may be significantly better even than real ObjC programs.
* Pointer packing for `Option<NSObject>`
* Smart pointer system, with support for `StrongCell`, `AutoreleasedCell` and `UnwrappedCell` (a pointer comparable to Swift's IUO)
* Subclassing directly from Rust
* (limited) support for mutability and exclusive references in imported types

Not yet implemented, but planned or possible:

* iOS support
* Exceptions (Debug-quality API available already, see `try_unwrap_void`

# Design limitations

This library **takes ObjC seriously**.  ObjC has many patterns that are difficult or unsafe to express in Rust.  As a consequence,
many APIs have been marked `unsafe` and require knowledge of both unsafe Rust and ObjC convention to use in a safe way.

A complete treatment of these topics is beyond the scope of any document, but some things to be aware of include:

1.  ObjC memory management patterns are "by convention", e.g. related to the name of an API or its historical use as known among ObjC programmers.
	Sound use of ObjC APIs requires you to correctly anticipate these conventions.
2.  It also requires ObjC APIs to be implemented correctly.  As ObjC is an unsafe language this may be of concern to Rust developers.
3.  ObjC exceptions are *generally* not to be handled, by analogy to Rust panics.  Also like Rust panics, they may frequently occur
	during development.  However *unlike* panics, ObjC exceptions are UB if they unwind into other languages so they may not reliably crash.
	Therefore, you must ensure they do not accomplish it, an admittedly difficult task.  It can be achieved with [bindings::try_unwrap_void], but this has some
	performance overhead that may be unacceptable for method calls, so whether or not to wrap your API that way is up to you.

	In not handling this, I followed Swift's design on this point, which faces a similar issue.  Presumably, they are more familiar
	with the tradeoffs than I am.

	However, Rust is substantially more likely to swallow debugging information when it encounters UB, so you may want to weigh your options,
	or at least be prepared to insert `try_unwrap` for debugging purposes.
