use crate::bindings::{GuaranteedMarker,RawMarker};
///Marks that a given type is an objc type, e.g. its instances are an objc object.
///This is the case for classes, but also for protocols.
///
/// Conforming types can be auto-declared via [objc_instance!] macro.
///
pub unsafe trait ObjcInstance  {
    ///Create an instance from a [GuaranteedMarker], which is a typed pointer.
    ///
    /// The pointer type must be `Self`.
    unsafe fn new(marker: GuaranteedMarker<Self>) -> Self;

    /// Returns a reference to the underlying marker.
    ///
    /// This function does not move out of self because it may allow multiple mutable references, which is plausibly unsafe although it's debatable.  (see [objc_instance!()#Mutability]).
    fn marker(&self) -> &GuaranteedMarker<Self>;

    ///Mutable variant of `marker`.
    fn marker_mut(&mut self) -> &mut GuaranteedMarker<Self>;
}

///Behavior we define for any `ObjcInstace`
pub trait ObjcInstanceBehavior {
    ///Clones the underlying ObjcInstance.
    ///
    /// This is unsafe for several reasons:
    /// 1.  While cloning pointers is generally safe, we consider it unsafe in this library.
    ///     See the comment for [objc_instance!()] for a longer explanation.
    /// 2.  This may allow multiple mutable references, which is plausibly unsafe although it's debatable.  (see [objc_instance!()#Mutability]).
    unsafe fn unsafe_clone(&self) -> Self;

    ///Casts the type to another type.
    ///
    /// In practice this is often combined with `unsafe_clone`.
    ///
    /// # Safety
    /// There is no guarantee that the source type is compatible with the destination type.
    unsafe fn cast<R : ObjcInstance>(self) -> R;
}

impl<T: ObjcInstance> ObjcInstanceBehavior for T {

    unsafe fn unsafe_clone(&self) -> Self {
        Self::new(self.marker().unsafe_clone())
    }
    unsafe fn cast<R: ObjcInstance>(self) -> R {
        R::new(self.marker().unsafe_clone().cast())
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

ObjC types are written as a `struct`.  On the calling side, these work as you would expect.

```
# struct NSExample;
# impl NSExample {
# fn new() -> Self { NSExample{} }
# fn instanceMethod(&self)  { }
# }
let example: NSExample = NSExample::new();
example.instanceMethod();
```

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

This raises an important question though.  What exactly *is* an `NSObject`?  What are the fields, can it be `:Sized`,
how do we allocate it?  Well
you can go [diving in the internals](https://mikeash.com/pyblog/friday-qa-2009-03-13-intro-to-the-objective-c-runtime.html) if you
like, but TL;DR these are private implementation details of the objc runtime and they change from year to year. There is no
practical way to represent them in the Rust typesystem with the memory semantics Rust expects.

There are some awkward ways around Rust semantics, such as [std::pin].  However as a practical matter pinning
 literally everything is awkward, there's not much you could "do"
with an `NSObject` even if you could represent one, or at least not much to do that would be wise, sane, or
continue to work next year.

ObjC violates key assumptions Rust programmers normally have, such as memory safety,
exclusive access/aliasing, etc.  This conflict is largely addressed by the design described
in the next section.  However, there is no way to do it totally, so binding authors
will need to carefully consider some of these details when they write bindings.  I describe
these tradeoffs in the following sections.

For our purposes though, `objc_type!{struct NSObject}` fulfills 3 key objectives:

1.  It provides a container in which to write bindings for an ObjC type with the same or similar name
2.  It provides a type (sometimes called the *marker type*) which we can use as a standin for the ObjC type, in the Rust typesystem.  This allows
Rust programs to be typesafe when they work with ObjC objects
3.  It allows programmers to call functions and get familiar behavior like [std::fmt::Display].

# Internal structure

The [ObjcInstance] trait does not impose any particular requirements on the memory layout of instance types.
For this reason a bindings author could use any approach, including approaches based on [std::pin] to mark
 ObjC objects if desired.

This macro, however, has a view:  [objc_instance!] is a struct with one field of [GuaranteedMarker<Self>], which
primarily wraps `*mut Self`.

This means [objc_instance!] actually declares a *reference type*.  The reference is "built in" to the owned value. That is,
 `&NSObject` is two pointers:  a) the reference to `struct NSObject`, and b) the reference *inside* an owned `struct NSObject`,
 to some actual ObjC object.

## Internal structure design

This strategy sounds odd, but it boils down to a typsystem difference between Rust and ObjC.  Since the people
knowing both languages might be few, I will try to explain the problem in some detail.

In Rust, value types are "fundamental", and reference types are "bolted on", e.g. via a wrapper like [std::boxed::Box].  This means
that pretty much every instance can be a value either because it already is or you unwrapped it somehow.

In ObjC, for the most part it's all reference types, and unwrapping to a value is somewhere between undocumented and unsound.
It's spiritually similar to a Rust DST, it's an opaque type that you can't move/copy and as Rust devs know,
somewhat useless without `&`, but for a whole language.

In fact, I considered using `&SomeDST` for this.  The challenge is that, well, the rest of Rust assumes you didn't.  For example,

```compile_fail
struct DST ([u8]);
fn make_dst() -> & 'mystery_lifetime DST { todo!("Somehow get a reference") }
impl DST {
    ///Call some external deallocator and make sure we don't use the value again
   fn dealloc(self) { todo!() }
   //error:    ^ doesn't have a size known at compile-time
}
```

Well duh.  The problem is that Rust overloads the semantics of a consuming function, with stack values.  But ObjC
objects cannot be stack values, so you cannot have consume semantics.

Meanwhile in ObjC, you have a language built on reference semantics.  One consequence is that many (though not all)
of the `&` and `*` are elided, producing bare code like `id foo = [[NSObject alloc] init];` where `foo` is really some pointer
type.  When you write that code, you start to think of references as not as an indirection for values, but as a first-class
type themselves.  Soon you start to say things like "owned reference" which sound like nonsense to the Rust typesystem.

In fact, I implemented that nonsense.  `objc_type!{struct NSObject}` is `NSObject` the "owned reference".  This
is not a total claim to the underlying objc object, which really belongs to the objc runtime.  It is however
a total claim to this reference, which if it turns out to be the only reference, really is owning the object.

In practice, objects are usually owned by a [objr::bindings::StrongCell] or [objr::bindings::AutoreleasedCell] which
handle `retain`/`release`/`autorelease` calls when references are destroyed according to the usual ObjC memory rules.

That leaves shared reference `&NSObject` and "exclusive" reference `&mut NSObject`.  The former is used for functions
thought not to mutate the `NSObject`, the latter used for operations thought *to* mutate the `NSObject`.  The notion
of mutation is somewhat different in Rust than ObjC, see the section on [objc_instance!()#Mutability].

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

The type is declared `#[repr(transparent)]` and can be transmuted with an objc pointer.
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
and other objects all have references to your objects, and may do such nonsensical mutations
as *adding new methods to objects at runtime*.  Therefore, forbidding mutation of shared
references, as is typical in Rust, is pretty much a non-starter for most ObjC work.
At the same time, handing out mutable references to every Rust caller who asks seems completely shady to Rust developers.

Instead, `NSExample`-references (that is, `&NSExample` or `&mut NSExample`) are "cell-scoped".  For
a particular `StrongCell<NSExample>` you may have exactly 1 exclusive/`&mut` reference, or alternatively,
unlimited borrow/`&` references.  As discussed in [objc_instance!()#Internal structure design] an `&mut NSExample`
is really a reference *reference*.

**You can create additional mutable references by creating a new cell to the same object.**

The latter may seem totally unexpected to Rust developers, and may remind them of `UnsafeCell` for
example.  However, in my opinion there are some key differences:

1.  Unlike in Rust, the ObjC pointer can be obtained or mutated various ways without unwrapping the cell, such as
    by asking the ObjC runtime, calling some ObjC method that returns it or unexpectedly mutates memory, etc.
    In practice, this is likely to happen, regardless of how the library models it in the Rust typesystem.

    No checking on the cell could solve this problem.  Alternatively, various ways to add checking
    would introduce performance cost without fully solving the issue.
2.  As discussed earlier, the memory layout of [objc_instance!] is really some pointer.  When you create a new `StrongCell`,
    the pointer is copied, meaning you never *actually* broke Rust's aliasing rules by having two mutable references
    to the same Rust struct.
3.  The 'pointer' is generally never mutated anyway, so the whole distinction between `&mut` and `&` is totally arbitrary.
4.  ObjC's notion of mutation is quite different than Rust's and is not totally bridgeable.  See the section on [objc_instance!()#ObjC mutation].

If it's never mutated, why have immutable/mutable types at all?  It provides an imperfect representation of the mutability
of the actual non-pointer ObjC instance into the Rust typesystem, as we discuss in the next section.

Keep in mind that these justifications may not apply to all [ObjcInstance] instances, only ones declared with this macro.

### ObjC mutation

A brief digression of mutation in Rust and mutation in ObjC.  Like with the values/references issue, Rust takes the view that mutation is
something "bolted on" to an existing type, e.g. `let f = mut Foo()` vs `let f = Foo()` both involve type `Foo`.

In ObjC however, you are likely to have completely distinct types `NSArray` and `NSMutableArray`, with distinct implementations.  There may
be some internal mechanism where distinct types share the same memory, but it's not guaranteed.  There is also no guarantee that
`NSArray` the "immutable" type does not actually do some mutation secretly, or that if you have an "NSArray" it's not really a duck-typed
`NSMutableArray` which happens to implement all `NSArray` methods.

At a high level though, many of the Rust practices around exclusive mutation are also good practices for ObjC objects.
In my opinion, importing a guess of whether some method mutates helps programmers avoid common bugs.

However, this information is a lot more advisory than it is for classic Rust types.  Extra care is required
for implementing `Send`, `Sync`, or otherwise moving ObjC types across a thread boundary.

# Programming model

Although `struct NSExample` contains a pointer, as a practical matter it stands in for the real ObjC
type we are not allowed to store inline.  What does this mean?  Well,

```
# use objr::bindings::*;
# struct NSExample;
impl NSExample {
    fn name(& self) -> AutoreleasedCell<'_, NSString> { todo!() }
    fn setName(&mut self, name: &str) { todo!() }
}
```

This means that `name` looks like (and is used in Rust like) an immutable function,
and `setName` looks like (and is used like) an immutable function.

That is to say

```
# use objr::bindings::*;
# objc_instance! {
#     pub struct NSExample;
# }
#     fn getExample() -> StrongCell<NSExample> {
#        unsafe{
#        let unwrapped_cell: UnwrappedCell<NSExample> = UnwrappedCell::new(GuaranteedMarker::dangling());
#        unwrapped_cell.assuming_retained()
#        }
#     }
# impl NSExample {
#  fn name(&self) {}
#  fn setName(&self, str: &str) {}
# }
let foo: StrongCell<NSExample> = getExample();
foo.name(); //deref to `&NSExample`
foo.setName(&"test"); //deref to `&mut NSExample`

let foo_ref = &*foo; //deref to `&NSExample`
foo_ref.name(); //ok
foo_ref.setName(&"test"); //not allowed on immutable `foo_ref`
foo.setName(&"test"); //not allowed if immutable borrow foo_ref is around
```

While this looks very much like plausible standard Rust behavior, it's important to point out that
the mutability is merely a loose contract.  In practice, ObjC may mutate objects for any reason,
or your binding author may have simply misread the documentation.  Therefore, consider
method annotations as a nice gesture rather than a legally binding document.  This may
be slightly unusual to Rust developers.

Finally, like you'd expect, owned functions can be used to invalidate the pointer.  This is used internally
to implement `release`

```
# struct MyExample; impl MyExample {
fn release(self) { todo!() }
# }
```

Following modern ObjC practice this function is not exposed as public API, but if you could
call it, it would move `NSExample` out of the cell, rendering the cell invalid, which is
what we want if the object is deallocated.  Keep in mind though that the cell's `Drop` implementation
will still run, which may incidentally try to dereference the ObjC object.


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
        $pub struct $objctype(::objr::bindings::GuaranteedMarker<Self>);
    };
}

///Defines some behavior on `Option<&ObjcInstance>`
pub trait OptionalInstanceBehavior<Deref> {
    ///Gets a marker for the option.  If `self` is `nil`, `RawMarker` will be `null`, otherwise it will be the underlying marker.
    fn marker(&self) -> RawMarker<Deref>;
}

impl<T: ObjcInstance> OptionalInstanceBehavior<T> for Option<&T> {
    fn marker(&self) -> RawMarker<T> {
        use crate::performselector::PerformablePointer;
        match self {
            Some(o) => unsafe{ RawMarker::new(o.marker().ptr()) },
            None => RawMarker::nil()
        }
    }
}