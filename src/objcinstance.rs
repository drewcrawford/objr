use std::ptr::NonNull;
use crate::bindings::{StrongCell, AutoreleasedCell};
use crate::autorelease::ActiveAutoreleasePool;

///Marks that a given type is an objc type, e.g. its instances are an objc object.
///This is the case for classes, but also for protocols.
///
/// Conforming types can be auto-declared via [objc_instance!] macro. todo
///
pub unsafe trait ObjcInstance  {
}



#[repr(transparent)]
#[derive(Debug)]
pub struct NonNullImmutable<T: ?Sized>(NonNull<T>);

impl<T: ObjcInstance> NonNullImmutable<T> {
    ///Assumes the object has been retained and converts to a StrongCell.
    ///
    /// # Safety:
    /// If the object is not +1 already, this will UB
    pub unsafe fn assuming_retained(self) -> StrongCell<T> {
        StrongCell::assuming_retained(self)
    }
    ///Assumes the object has been autoreleased and converts to an AutoreleasedCell.
    ///
    /// # Safety:
    /// If the object is not autoreleased already, this will UB
    pub unsafe fn assuming_autoreleased(self, pool: &ActiveAutoreleasePool) -> AutoreleasedCell<'_, T> {
        AutoreleasedCell::assuming_autoreleased(self, pool)
    }
    pub(crate) fn as_ptr(&self) -> *const T {
        self.0.as_ptr()
    }
    pub(crate) unsafe fn assuming_nonnil(ptr: *const T) -> Self {
        Self(NonNull::new_unchecked(ptr as *mut T))
    }

}

///Behavior we define for any `ObjcInstace`
pub trait ObjcInstanceBehavior {

    ///Casts the type to another type.
    ///
    /// In practice this is often combined with `unsafe_clone`.
    ///
    /// # Safety
    /// There is no guarantee that the source type is compatible with the destination type.
    unsafe fn cast<R : ObjcInstance>(underlying: *const Self) -> *const R;

    ///Assuming the pointer is non-nil, returns a pointer type
    unsafe fn assuming_nonnil(ptr: *const Self) -> NonNullImmutable<Self>;

    ///Allows you to call [perform] from a nonmutating context.
    ///
    /// This function should not be used for general-purpose pointer casting.
    ///
    /// # Safety
    /// This is only safe when the underlying objc method does not mutate its contents.  See [objc_instance#Mutability] for details.
    unsafe fn assuming_nonmut_perform(&self) -> *mut Self;
}

impl<T: ObjcInstance> ObjcInstanceBehavior for T {
    unsafe fn cast<R: ObjcInstance>(underlying: *const Self) -> *const R {
        underlying as *const _ as *const R
    }
    unsafe fn assuming_nonnil(ptr: *const Self) -> NonNullImmutable<Self> {
        NonNullImmutable(NonNull::new_unchecked(ptr as *mut Self))
    }
    unsafe fn assuming_nonmut_perform(&self) -> *mut Self {
        self as *const Self as *mut Self
    }

}

/**
Defines a struct (binding) for a specific ObjC type.  This doesn't assume the type is a class, if it is a class consider [objc_class!].

The type will automagically conform to [ObjcInstance].

# Example

```
#![link(name="Foundation",kind="framework")]
use objr::bindings::*;
objc_instance! {
    pub struct NSExample;
}
```

# The problem

ObjC and Rust disagree on a great many things.  Rust prefers
stack allocation, ObjC types are all heap-allocated.  Rust expects static
lifetime proofs, ObjC has significant runtime memory management.  Rust expects
 to know things like whether a reference is exclusive or not that ObjC withholds.
 And of course silly things like `snake_case_methods()` vs `camelCaseMethods`.

This library is in the unenviable position of trying to please everybody, which cannot really
be done, but meanwhile I have software to write, so here is the Grand Compromise in use around here.

# Representing ObjC types

ObjC types are declared as 'opaque' types.  While these types technically have a memory layout in Rust,
the memory layout is not the same as the corresponding ObjC layout.  Therefore, such types are "effectively" DSTs,
and cannot be stored on the stack or dereferenced.  For more information, see unstable feature [RFC 1861](https://rust-lang.github.io/rfcs/1861-extern-types.html),
and we implement some "similar" technique in stable Rust.

In short, this is the type situation for Rust:

1. `example: NSExample`.  This type effectively is not instantiable, but you can imagine it as the object "owned" by the ObjC runtime.  Since you can't
    move it out of the runtime you cannot use instances of it.  There are some gotchas of this type even without constructing it, e.g. its memory
    layout may be different than the situation in ObjC really would be for example.
2.  `example: *mut NSExample`.  What an ObjC (non-ARC) developer would think of as a normal reference, a Rust raw pointer.
3.  `example: *const NSExample`.  What an ObjC (non-ARC) developer would think of as an immutable reference, as some mutable methods may not be available.
4.  `example: &mut NSExample` 2, but checked by the borrowchecker. One limitation of this type is it is UB if you make it `nil`, so consider modeling with `Option`.  It is somewhat unusual for ObjC to use objects derived from the lifetime of another object (rather than an independently-tracked object with one of the [objectpointers], so this primarily comes up in the context of inner pointers.
5.  `example: &NSExample` 3, but checked by the borrowchecker.  One limitation of this type is it is UB if you make it `nil`, so consider modeling with `Option`.  It is somewhat unusual for ObjC to use objects derived from the lifetime of another object (rather than an independently-tracked object with one of the [objectpointers], so this primarily comes up in the context of inner pointers.

ARC is its own topic, which in Rust is handled by various smart pointers.  See [objectpointers] for details on the pointer types.

For writing bindings, a stub implementation is:

```
# use objr::bindings::*;
objc_instance! {
    pub struct NSExample;
}
impl NSExample {
    fn new() -> Self { todo!() }

    //I generally follow ObjC syntax for method names, although I'm not gonna tell you what to do.
    #[allow(non_snake_case)]
    fn instanceMethod(&self) { todo!() }
}
```

ObjC violates key assumptions Rust programmers normally have, such as memory safety,
exclusive access/aliasing, etc.  This conflict is largely addressed by the design described
in the next section.  However, there is no way to do it totally, so binding authors
will need to carefully consider some of these details when they write bindings.  I describe
these tradeoffs in the following sections.

For our purposes though, `objc_instance!{struct NSObject}` fulfills 3 key objectives:

1.  It provides a container in which to write bindings for an ObjC type with the same or similar name
2.  It provides a type (sometimes called the *marker type*) which we can use as a standin for the ObjC type, in the Rust typesystem.  This allows
Rust programs to be typesafe when they work with ObjC objects
3.  It allows programmers to call functions and get familiar behavior like [std::fmt::Display].

## Internal structure design

This involves a typsystem difference between Rust and ObjC.  Since the people
knowing both languages might be few, I will try to explain the problem in some detail.

In Rust, value types are "fundamental", and reference types are "bolted on", e.g. via a wrapper like [std::boxed::Box], pointer types like `*const T`, etc.  This means
that pretty much every instance can be a value either because it already is or you unwrapped it somehow.

In ObjC, for the most part it's all reference types, and unwrapping to a value is somewhere between undocumented and unsound.
It's spiritually similar to a Rust DST, it's an opaque type that you can't move/copy and as Rust devs know,
somewhat useless without `&`, but for a whole language.

# Safety

Strictly speaking, you can clone, copy, send, etc., some secret pointer field without any safety problems.
The trouble comes when you go to use this pointer, a.k.a. literally any ObjC method call.

However, this would mean every call is unsafe Rust, which is in my opinion completely impractical,
and also not the way ObjC wants to be used.  ObjC imagines the pointer part as the danger
and the use to be safe.

This library tries to interpret ObjC's worldview in a way mostly palatable to Rust programmers.
Notably, all the ways to create pointers are either checked, or some `unsafe` function, whereas
some of the ways to *use* pointers assume they were checked when they were created.  This should
still provide safety guarantees for safe Rust code, with the proviso that if you screw up your
unsafe code, it may be harder to debug because there may be more distance between you and the original mistake.

Partly due to this, if you are doing "dangerous pointer stuff", you may discover it is even
more dangerous in this library than usual, so you may want to tread carefully.

## FFI-safety

objc-instance type is declared `#[repr(C)]` and pointers to it are valid objc pointers.
This is primarily so that the this macro can be used for subclassing and passed into a rust fn.

Note that in order to use subclassing, you need to use the [objc_subclass!] macro.

## Memory management

You can find out more about in the documentation
for [StrongCell](objr::bindings::StrongCell) or [AutoreleasedCell](objr::bindings::AutoreleasedCell).  Suffice it to say here that use of either cell
largely protects you from dangling pointers, but ObjC is pretty much one giant `unsafe` block, so
it's always possible the ObjC side accidentally frees your memory or even does it on purpose.  Care
must be taken in exposing functions to Rust, and in cases that require extra care,
I mark such functions `unsafe`.

## Nullability

Another issue is that in ObjC,whether APIs are "nullable" (void pointer, treated in practice
like Rust `Option<T>::None`) or "non-nullable". Depending on your function this information may appear in headers,
buried in documentation, be widely assumed by ObjC programmers or be buggy and do the opposite thing in rare cases.

The convention in ObjC is to assume things are as specified and UB if it winds up differently, a convention
I generally follow.  This comfort with UB will seem incredibly stupid to Rust developers but the fact is
that a) nil-checking every call just to panic is a high cost for a low benefit, b) since everyone knows
a bug causes UB, the APIs mostly succeed in not having bugs, c) unexpected `nil` is only one of about a bajillion
UBs that could occur in an ObjC call, so it is in my opinion mostly security theater to look for it.

Of course there is an API to check nullability, [objr::bindings::GuaranteedMarker::try_unwrap()], so if you disagree
with me on this topic you are welcome to use it for all your bindings.

## Mutability

The fact is that every ObjC object is behind many shared mutable references.  The runtime itself, your code,
and other objects all have mutable references to your objects, and may do such nonsensical mutations
as *adding new methods to objects at runtime*.  Therefore, forbidding mutation of shared
references, as is typical in Rust, is pretty much a non-starter for most ObjC work.
At the same time, handing out multiple mutable references seems very unsafe from a Rust POV.

Instead, I generally model what I suspect are mutable methods with `&mut` and immutable methods with `&`.  For example,
you can access an array while holding bare references to objects within the array, but if you replace an object in the array, that is forbidden while
holding bare references, since one of them may be released by the array during the mutation.

This generally works as Rust developers expect, with the proviso that it relies on some guesswork on my part.  In practice,
there is no law that ObjC can't release your references internally if you call some "immutable" method, so maybe your safe code
can do UB.  I consider these to be bugs, and please file them if you encounter them, but effectively, the chances of them happening
are higher, because the details to do it accurately are undocumented. But in my view, it is better to find and fix such situations
than make every call unsafe everywhere.

One thing that *is* marked unsafe is cloning pointers. **You can create additional mutable references by creating
a new cell to the same object.**

### ObjC mutation

A brief digression of mutation in Rust and mutation in ObjC.  Like with the values/references issue, Rust takes the view that mutation is
something "bolted on" to an existing type, e.g. `let f = mut Foo()` vs `let f = Foo()` both involve type `Foo`.

In ObjC however, you are likely to have completely distinct types `NSArray` and `NSMutableArray`, with distinct implementations.  There may
be some internal mechanism where distinct types share the same memory, but it's not guaranteed.  There is also no guarantee that
`NSArray` the "immutable" type does not actually do some mutation secretly, or that if you have an "NSArray" it's not really a duck-typed
`NSMutableArray` which happens to implement all `NSArray` methods.

Extra care is required for implementing `Send`, `Sync`, or otherwise moving ObjC types across a thread boundary.

 */
#[macro_export]
macro_rules! objc_instance  {
    (
        $(#[$attribute:meta])*
        $pub:vis
        struct $objctype:ident;
    ) => {
        $(#[$attribute])*
        #[repr(transparent)]
        #[derive(::objr::bindings::ObjcInstance,Debug)]
        $pub struct $objctype(core::ffi::c_void);
    };
}

///Defines some behavior on `Option<&ObjcInstance>`
pub trait OptionalInstanceBehavior<Deref> {
    ///Gets a pointer for the option.  If `self` is `nil`, the pointer will be `null`, otherwise it will be the underlying reference.
    fn as_ptr(&self) -> *const Self;
}

impl<T: ObjcInstance> OptionalInstanceBehavior<T> for Option<&T> {
    fn as_ptr(&self) -> *const Self {
        self as *const Self
    }
}