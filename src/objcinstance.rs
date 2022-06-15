use std::ptr::NonNull;
use crate::arguments::Arguable;
use crate::bindings::{StrongCell, AutoreleasedCell, StrongLifetimeCell, StrongMutCell};
use crate::autorelease::ActiveAutoreleasePool;

///Marks that a given type is an objc type, e.g. its instances are an objc object.
///This is the case for classes, but also for protocols.
///
/// # Stability
/// It is not stable API to implement this trait yourself.  Instead, declare a conforming
/// type via [objc_instance!] macro.
///
pub trait ObjcInstance: Arguable {}


///A nonnull, but immutable type.  This allows various optimizations like pointer-packing `Option<T>`.
///
#[repr(transparent)]
#[derive(Debug)]
pub struct NonNullImmutable<T: ?Sized>(NonNull<T>);

impl<T: ObjcInstance> NonNullImmutable<T> {
    pub(crate)  fn from_reference(ptr: &T) -> Self {
        unsafe{ NonNullImmutable::assume_nonnil(ptr) }
    }
    ///Assumes the object has been retained and converts to a StrongCell.
    ///
    /// # Safety
    /// You must guarantee each of the following:
    /// * Object was retained (+1)
    /// * Object is not deallocated
    /// * Object was initialized
    /// * Object is 'static, that is, it has no references to external (Rust) memory.
    /// If this is not the case, see [NonNullImmutable::assume_retained_limited].
    pub unsafe fn assume_retained(self) -> StrongCell<T> {
        StrongCell::assume_retained(self.0.as_ref())
    }

    ///Assumes the object has been retained and converts to a StrongLifetimeCell.
    ///
    /// # Safety
    /// You must guarantee each of the following:
    /// * Object was retained (+1)
    /// * Object is not deallocated
    /// * Object was initialized
    /// * That the object can remain valid for the lifetime specified.  e.g., all "inner pointers" or "borrowed data" involved
    /// in this object will remain valid for the lifetime specified, which is unbounded.
    /// * That all objc APIs which end up seeing this instance will either only access it for the lifetime specified,
    ///   or will take some other step (usually, copying) the object into a longer lifetime.
    pub unsafe fn assume_retained_limited<'a>(self) -> StrongLifetimeCell<'a, T> where T: 'a {
        StrongLifetimeCell::assume_retained_limited(self.0.as_ref())
    }
    ///Assumes the object has been autoreleased and converts to an AutoreleasedCell.
    ///
    /// # Safety:
    /// You must guarantee each of the following:
    /// * Object is autoreleased already
    /// * Object is not deallocated
    /// * Object was initialized
    pub unsafe fn assume_autoreleased<'a>(self, pool: &'a ActiveAutoreleasePool) -> AutoreleasedCell<'a, T> {
        AutoreleasedCell::assume_autoreleased(self.as_ref(), pool)
    }
    ///Converts to a raw pointer
    pub(crate) fn as_ptr(&self) -> *const T {
        self.0.as_ptr()
    }
    ///Assumes the passed pointer is non-nil.
    ///
    /// # Safety
    /// You must guarantee each of the following:
    /// * Pointer is non-nil
    /// * Points to a valid objc object of the type specified
    pub(crate) unsafe fn assume_nonnil(ptr: *const T) -> Self {
        Self(NonNull::new_unchecked(ptr as *mut T))
    }

    ///Dereferences the inner pointer.
    ///
    /// # Safety
    /// You must guarantee each of the following
    /// * Object is not deallocated
    /// * Object will not be deallocated for the lifetime of `self` (e.g., the lifetime of the returned reference)
    /// * Object was initialized
    unsafe fn as_ref(&self) -> &T {
        self.0.as_ref()
    }

    ///Retains the inner pointer and converts to [StrongCell]
    ///
    /// # Safety
    /// You must guarantee each of the following
    /// * Object is not deallocated
    /// * object was initialized
    pub unsafe fn retain(&self) -> StrongCell<T> {
        StrongCell::retaining(self.as_ref())
    }

}
///Behavior we define for any [ObjcInstance].
pub trait ObjcInstanceBehavior {

    ///Casts the type to another type.
    ///
    /// # Safety
    /// There is no guarantee that the source type is compatible with the destination type.
    unsafe fn cast<R : ObjcInstance>(&self) -> &R;

    ///Casts the type to another type.
    ///
    /// # Safety
    /// There is no guarantee that the source type is compatible with the destination type.
    /// To the extent that you create two pointers pointing to the same instance,
    /// this may be UB
    unsafe fn cast_mut<R: ObjcInstance>(&mut self) -> &mut R;

    ///Assuming the pointer is non-nil, returns a pointer type.
    ///
    /// The opposite of this function is [Self::nullable].
    ///
    /// # Safety
    /// You must guarantee each of the following:
    /// * Pointer is non-nil
    /// * Points to a valid objc object of the type specified
    unsafe fn assume_nonnil(ptr: *const Self) -> NonNullImmutable<Self>;

    ///Safely casts the object to an `Option<NonNullImmutable>`.  Suitable for implementing nullable functions.
    fn nullable(ptr: *const Self) -> Option<NonNullImmutable<Self>>;

}

impl<T: ObjcInstance> ObjcInstanceBehavior for T {
    unsafe fn cast<R: ObjcInstance>(&self) -> &R {
        &*(self as *const _ as *const R)
    }
    unsafe fn cast_mut<R: ObjcInstance>(&mut self) -> &mut R {
        &mut *(self as *mut _ as *mut R)
    }
    unsafe fn assume_nonnil(ptr: *const Self) -> NonNullImmutable<Self> {
        NonNullImmutable(NonNull::new_unchecked(ptr as *mut Self))
    }

    fn nullable(ptr: *const Self) -> Option<NonNullImmutable<Self>> {
        if ptr.is_null() {
            None
        }
        else {
            //we checked this above
            Some(unsafe{ Self::assume_nonnil(ptr) })
        }
    }

}

///Helper for Option<NonNullable>
pub trait NullableBehavior {
    type T: ObjcInstance;
    ///Assumes the object has been autoreleased and converts to an Option<AutoreleasedCell>
    ///
    /// # Safety:
    /// You must guarantee each of the following:
    /// * Object (if any) is autoreleased already
    /// * Object (if any) is not deallocated
    /// * Object (if any) was initialized
    unsafe fn assume_autoreleased<'a>(self, pool: &'a ActiveAutoreleasePool) -> Option<AutoreleasedCell<'a, Self::T>>;
    ///Assumes the object has been retained and converts to a StrongCell.
    ///
    /// # Safety
    /// You must guarantee each of the following:
    /// * Object was retained (+1)
    /// * Object (if any) is not deallocated
    /// * Object (if any) was initialized
    unsafe fn assume_retained(self) -> Option<StrongCell<Self::T>>;

    ///Retains the inner pointer and converts to [StrongCell]
    ///
    /// # Safety
    /// You must guarantee each of the following
    /// * Object (if any) is not deallocated
    /// * object (if any) was initialized
    unsafe fn retain(self) -> Option<StrongCell<Self::T>>;

    ///Assumes the object has been retained and converts to a StrongLifetimeCell.
    ///
    /// # Safety
    /// You must guarantee each of the following:
    /// * Object (if any) was retained (+1)
    /// * Object (if any) is not deallocated
    /// * Object (if any) was initialized
    /// * That the object (if any) can remain valid for the lifetime specified.  e.g., all "inner pointers" or "borrowed data" involved
    /// in this object will remain valid for the lifetime specified, which is unbounded.
    /// * That all objc APIs which end up seeing this instance will either only access it for the lifetime specified,
    ///   or will take some other step (usually, copying) the object into a longer lifetime.
    unsafe fn assume_retained_limited<'a>(self) -> Option<StrongLifetimeCell<'a, Self::T>> where Self::T: 'a;
}
impl<O: ObjcInstance> NullableBehavior for Option<NonNullImmutable<O>> {
    type T = O;

    unsafe fn assume_autoreleased<'a>(self, pool: &'a ActiveAutoreleasePool) -> Option<AutoreleasedCell<'a, O>> {
        self.map(|m| m.assume_autoreleased(pool))
    }

    unsafe fn assume_retained(self) -> Option<StrongCell<Self::T>> {
        self.map(|m| m.assume_retained())
    }

    unsafe fn retain(self) -> Option<StrongCell<Self::T>> {
        self.map(|m| m.retain())
    }
    unsafe fn assume_retained_limited<'a>(self) -> Option<StrongLifetimeCell<'a, Self::T>> where Self::T: 'a {
        self.map(|m| m.assume_retained_limited())
    }
}

///Helper for Option<StrongCell>
pub trait NullableCellBehavior {
    type T: ObjcInstance;
    ///Converts to a mutable version.
    ///
    /// # Safety
    /// You are responsible to check:
    /// * There are no other references to the type, mutable or otherwise
    /// * The type is in fact "mutable", whatever that means.  Specifically, to whatever extent `&mut` functions are forbidden
    ///   generally, you must ensure it is appropriate to call them here.
    unsafe fn assume_mut(self) -> Option<StrongMutCell<Self::T>>;
}
impl<O: ObjcInstance> NullableCellBehavior for Option<StrongCell<O>> {
    type T = O;

    unsafe fn assume_mut(self) -> Option<StrongMutCell<Self::T>> {
        self.map(|p| p.assume_mut())
    }
}

/**
Defines a struct (binding) for a specific ObjC type.  This doesn't assume the type is a class, if it is a class consider [objc_class!].

The type will automagically conform to [objr::bindings::ObjcInstance], but will not conform to [objr::bindings::ObjcClass].

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

ObjC types are declared as 'opaque' Rust types.  While these types technically have a memory layout in Rust,
the memory layout is not the same as the corresponding ObjC layout.  Therefore, such types are "effectively" DSTs,
and cannot be stored on the stack or dereferenced.  For more information, see unstable feature [RFC 1861](https://rust-lang.github.io/rfcs/1861-extern-types.html).
We implement a "similar" feature in stable Rust.

In short, this is the type situation for Rust code:

1. `example: NSExample`.  This type effectively is not instantiable, but you can imagine it as the object "owned" by the ObjC runtime.  Since you can't
    move it out of the runtime you cannot use instances of it.  There are some gotchas of this type even without constructing it, e.g. its memory
    layout may be different than the situation in ObjC really would be for example.
2.  `example: *mut NSExample`.  What an ObjC (non-ARC) developer would think of as a normal reference, a Rust raw pointer.
3.  `example: *const NSExample`.  What an ObjC (non-ARC) developer would think of as an immutable reference, as some mutable methods may not be available.
4.  `example: &mut NSExample` 2, but checked by the borrowchecker. One limitation of this type is it is UB if you make it `nil`, so consider modeling with `Option`.  While this type is appropriate for parameters, it is somewhat unusual for return values as ObjC is reluctant to relate object lifetimes to each other.
5.  `example: &NSExample` 3, but checked by the borrowchecker.  One limitation of this type is it is UB if you make it `nil`, so consider modeling with `Option`.  hile this type is appropriate for parameters, it is somewhat unusual for return values as ObjC is reluctant to relate object lifetimes to each other.

ARC is its own topic, which in Rust is handled by various smart pointers.  See [objr::bindings::StrongCell] and [objr::bindings::AutoreleasedCell] for details on the pointer types.

Let's stub out a binding for some ObjC type `NSExample`:

```
//we're writing bindings
use objr::bindings::*;
///NSExample is some objc instance (such as a protocol or similar).
//If it were a class, consider objc_class! for extra features.
objc_instance! {
    //declares a Rust struct for this type.
    //Note that there is no real connection to the actual objc type, you can name it anything.
    //The connection arises by casting some pointer to this type, such as the result of [PerformsSelector::perform].
    pub struct NSExample;
}
//We can write normal Rust functions on our type
impl NSExample {
    fn new() -> Self { todo!() }

    //I generally follow ObjC syntax for method names, although I'm not gonna tell you what to do.
    #[allow(non_snake_case)]
    fn instanceMethod(&self) { todo!() }
}
```
Declaring our type with the `objc_instance!` macro performs several tasks:

1.  It declares a type which we can use as a standin for the ObjC type, in the Rust typesystem.  This allows
Rust programs to be typesafe when they work with ObjC objects
2.  It provides a container in which to write bindings for the underlying ObjC type
3.  It allows end users to call methods and get familiar behavior like [std::fmt::Display].

# Safety

This library *intends* to follow the normal Rust safety guarantees, although there are a few areas that are
more risky than other libraries.

In general, ObjC is a giant ball of unsafe, opaque code.  If you are using this macro, or using something that uses this macro,
you are calling into that giant ball of unsafe code, and who knows if it's sound or not.

With that background, there are really two bad options:

a) Insert various runtime checks everywhere.  This is slow and stuff still slips through.
b) Just assume ObjC works as specified.  This is fast and more stuff slips through.

This library picks b.  Now we cover the topic with examples.


## FFI-safety

[ObjcInstance] type is declared `#[repr(C)]` and pointers to it are valid objc pointers.
So they can be passed directly to any method that expects an ObjC argument.  For example, C functions,
fucntions that implement subclassing inside Rust, etc.

Real ObjC objects can (usually) not be allocated on the stack.  This macro should prevent
owned pointers from being constructed (e.g. on the stack).

## Memory management

You can find out more about in the documentation
for [StrongCell](objr::bindings::StrongCell) or [AutoreleasedCell](objr::bindings::AutoreleasedCell).  Suffice it to say here that use of either cell
largely protects you from dangling pointers, but ObjC is pretty much one giant `unsafe` block, so
it's always possible the ObjC side accidentally frees your memory or even does it on purpose.

Return types for your binding require special consideration.  In general, ObjC memory rules are "by convention",
based on the method name, widely assumed by ObjC programmers, or be buggy and do the opposite thing in rare cases.

However, there are deep mysteries of ObjC not even known to most ObjC programmers.  In practice, the return type
you want is usually [StrongCell](objr::bindings::StrongCell), even in cases where the function is known to
be autoreleased (+0 convention).  Why is this?

In fact, the conventional model that ObjC methods return "either" +0 (autoreleased) or +1 (retain/create/copy) is out of date.
Most +0 methods don't return an autorelease object, but return the result of [`_objc_autoreleaseReturnValue`](https://clang.llvm.org/docs/AutomaticReferenceCounting.html#id63).
This obscure runtime function walks up the stack frame to inspect callers.  Callers that are "dumb" get the +0 object,
but smart callers can get a +1 object.

To be a smart caller, call a function like [`objr::bindings::PerformsSelector::perform_autorelease_to_retain`].  This will promote your +0 pointer to +1,
which can then be passed to [StrongCell](objr::bindings::StrongCell).

## Nullability

Another issue is that in ObjC, whether APIs are "nullable" (return a null pointer, usually called 'nil', treated in practice
like Rust `Option<T>::None`) is also by convention.

Unfortunately, in Rust it is UB to construct a reference to null.  Therefore a choice needs to be made about
whether an ObjC pointer should be interepreted as `Option<&T>` or `&T` and the wrong one may UB.

In general, this library takes the view that ObjC functions are correctly implemented.  Therefore, when something is
documented or "widely known" to be nonnull we use `&T` without checking.  This follows the precedent of languages like Swift,
although Swift has had trouble with this too.  For more information, see [SR-8622](https://bugs.swift.org/browse/SR-8622).

## Mutability

The fact is that every ObjC object is behind many shared mutable references.  ObjC has no law against
mutating its own state at any time, and effectly all pointers in the language are always mutable.  This is undesireable
to Rust developers who may be used to holding inner references to a type and using the borrow checker to prove
that the type is not mutated during the lifetime of inner references.  This pattern of inner references
is substantially less likely for ObjC objects although it does crop up in the context of a few types.

ObjC does have a concept of mutability/immutability, through type pairs (like `NSString` vs `NSMutableString`).
This can be used to achieve some version of mutability guarantees, however `NSString` may do some "inner mutation" somewhere,
so as the basis for a Rust system it isn't great.

Instead, I have implemented `&` and `&mut` as orthogonal to `NSString` vs `NSMutableString`.  You can have `&mut NSString`
and `&NSMutableString`.

Methods that I have reason to suspect mutate the inner storage are declared `fn mutating(&mut self)`, while methods I think
do not are implemented `fn nonmutating(&self)`.  In practice, this means a lot of the `NSMutable` type methods are (`&mut`) and
the former are `&`.

This generally works as Rust developers expect, with the proviso that it relies on, yet again, convention.  In practice,
there is no law that ObjC can't release your references internally if you call some "immutable" method, so maybe your safe code
can do UB.  I consider these to be bugs, and please file them if you encounter them, but effectively, I think it's preferable
for "immutable" methods to be immutable, than for everything to be `&mut`.

There are some methods that can create "additional" `&mut` references to a type, these are declared `unsafe` because
they may be used to violate Rust's exclusive references.

## Exceptions

ObjC exceptions are analogous to Rust panics.  In practice they abort your program, there is technically some way to handle them
but nobody does, and the decision to support that is a very unfortunate design mistake that now lives on forever.

More unfortunately, encountering an ObjC exception in Rust is UB.  This is substantially worse than a normal abort,
because you may not even get a reasonable abort or error message.

Since these are primarily not intended to be handled, it is undesireable to try to catch them.  Instead, the recommended approach
is to validate arguments on the Rust side (such as with a Rust assert or panic) so that they won't encountered on the ObjC side.
Or alternatively, to mark bindings as `unsafe` when there is some suspicion that ObjC exceptions may occur and push the problem
into the caller.

There is a [objr::bindings::try_unwrap_void] function which can upgrade the UB to a hard abort.
This function is expensive and not recommended for general use, but it is useful for debugging when you get a weird crash
and need to see an exception print to understand what is wrong.

Having exceptions as UB is a bit scary.  Once again though, we are following in the footsteps of Swift which does something very
similar.  Unfortunately, Swift is better at wringing a proper error message out of the exception, even though it isn't totally
reliable either.

# Generic types
Both ObjC and Rust support generics, which are vaguely similar concepts.  However, ObjC's notion of generics is highly 'bolted
on top': it serves as a compile-time assertion that some function accepts or returns a particular type, but it does not
actually constrain the runtime behavior, not does specialization create a distinct type.

The best way to project this in Rust is to project the "bolted on top" model.  Therefore (and also for technical reasons), this
macro does not accept generic arguments, but [objc_instance_newtype] does.

## Multithreading

Types declared with this macro do not implement Send or Sync.

```compile_fail
fn test_not_send() {
    fn assert_send<S: Send>(s: &S) { }
    objc_instance! {
        pub struct Example;
    }
    fn init() -> &'static Example { todo!() }
    assert_send(init());
}
```

 */
#[macro_export]
macro_rules! objc_instance  {
    (
        $(#[$attribute:meta])*
        $pub:vis
        struct $objctype:ident;
    ) => {
        //Idea here is we don't allow the type to be constructed where it is declared.
        //Doing so would allow stack allocation.
        //By nesting inside a separate module, the inner field is private.
        ::objr::bindings::__mod!(no_construct,$objctype, {
            $(#[$attribute])*
            #[repr(transparent)]
            #[derive(::objr::bindings::ObjcInstance,Debug)]
            pub struct $objctype(core::ffi::c_void,
            //mark as non-send
            std::marker::PhantomData<*const ()>);
        });
        ::objr::bindings::__use!($pub no_construct,$objctype,$objctype);
    };
}

///Duplicate macro that does not emit debug.
///todo: maybe we should refactor this to avoid DIY?
macro_rules! objc_instance_no_debug  {
    (
        $(#[$attribute:meta])*
        $pub:vis
        struct $objctype:ident;
    ) => {
        //Idea here is we don't allow the type to be constructed where it is declared.
        //Doing so would allow stack allocation.
        //By nesting inside a separate module, the inner field is private.
        ::objr::bindings::__mod!(no_construct,$objctype, {
            $(#[$attribute])*
            #[repr(transparent)]
            #[derive(::objr::bindings::ObjcInstance)]
            pub struct $objctype(core::ffi::c_void,
            //mark as non-send
            std::marker::PhantomData<*const ()>);
        });
        ::objr::bindings::__use!($pub no_construct,$objctype,$objctype);
    };
}
pub(crate) use objc_instance_no_debug;


/**
Declares a newtype that wraps an existing objc instance type.

Downcasts to the raw type will be implemented for you.  Upcasts will not, implement them yourself with [objr::bindings::ObjcInstanceBehavior::cast()] if applicable.
```no_run
use objr::bindings::*;
objc_instance! {
    struct NSExample;
}
objc_instance_newtype! {
    struct SecondExample: NSExample;
}
let s: &SecondExample = todo!();
let e: &NSExample = s.into();

let s: &mut SecondExample = todo!();
let e: &mut NSExample = s.into();
```

unlike [objc_instance!], this macro supports generic types, allowing you to wrap some other type with generics bolted on top.

At the moment, restrictions on generic arguments are not supported at the type level, but you can add them on your own impl blocks

```
use objr::bindings::*;
objc_instance! {
    struct NSExample;
}
objc_instance_newtype! {
    struct SecondExample<A,B>: NSExample;
}
//further restriction
impl<A: PartialEq,B: PartialEq> SecondExample<A,B> { }
```
*/
#[macro_export]
macro_rules! objc_instance_newtype {
    (
        $(#[$attribute:meta])*
        $pub:vis
        struct $newtype:ident $(<$($T:ident),+>)? : $oldtype:ident;
    ) => {
        ::objr::bindings::__mod!(no_construct,$newtype, {
            $(#[$attribute])*
            #[repr(transparent)]
            #[derive(Debug)]
            pub struct $newtype$(<$($T),+>)? (core::ffi::c_void, $($(std::marker::PhantomData<$T>),+)? );
        });
        ::objr::bindings::__use!($pub no_construct,$newtype,$newtype);
        unsafe impl $(<$($T),+>)? Arguable for $newtype $(<$($T),+>)? {}
        impl $(<$($T),+>)? ObjcInstance for $newtype $(<$($T),+>)? {}
        impl<'a,$($($T),*)?> From<&'a $newtype $(<$($T),+>)? > for &'a $oldtype {
            fn from(f: &'a $newtype $(<$($T),+>)?) -> &'a $oldtype {
                unsafe{ f.cast() }
            }
        }
        impl<'a,$($($T),*)?> From<&'a mut $newtype $(<$($T),+>)? > for &'a mut $oldtype {
            fn from(f: & 'a mut $newtype $(<$($T),+>)?) -> &'a mut $oldtype {
                unsafe{ f.cast_mut() }
            }
        }

    }
}


///Defines some behavior on `Option<&ObjcInstance>`
pub trait OptionalInstanceBehavior<Deref> {
    ///Gets a pointer for the option.  If `self` is `nil`, the pointer will be `null`, otherwise it will be the underlying reference.
    fn as_ptr(&self) -> *const Deref;
}

impl<T: ObjcInstance> OptionalInstanceBehavior<T> for Option<&T> {
    fn as_ptr(&self) -> *const T {
        if let Some(&s) = self.as_ref() {
            s
        }
        else {
            std::ptr::null()
        }
    }
}