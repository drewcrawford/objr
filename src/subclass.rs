#[macro_export]
#[doc(hidden)]
macro_rules! __objc_sublcass_implpart_method_prelude {
    ($MethodT:ident,$MethodListT:ident) => {
        #[repr(C)]
        struct $MethodT {
            //in objc-runtime.h this is declared as SEL
            name: *const u8,
            types: *const u8,
            imp: *const c_void
        }

        //need a variably-sized type?  Const generics to the rescue!
        #[repr(C)]
        struct $MethodListT<const SIZE: usize> {
            //I think we place 24 in here, although high bits may be used at runtime?
            magic: u32,
            //method count
            count: u32,
            methods: [MethodT; SIZE],
        }

    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __objc_subclass_implpart_a {
    ($pub:vis,$identifier:ident,$objcname:ident,$superclass:ident,
    //these ivars are imported from external scope to achieve macro hygiene
    $CLASS_NAME:ident,
    $NSSUPER_CLASS:ident,$OBJC_EMPTY_CACHE:ident) => {
        use core::ffi::c_void;
        objr::bindings::__mod!(subclass_impl_,$identifier, {
            #[repr(C)]
            pub struct IvarListT {
                //some dispute about whether this is the size of ivar_list_t,
                //a magic number, or both.  In practice it's 32
                pub magic: u32,
                pub count: u32,
                //todo: support multiple ivars.  For now, just inline the contents of an ivar, which are
                //points to FRAGILE_BASE_CLASS_OFFSET
                pub offset: *const u32,
                pub name: *const u8,
                pub r#type: *const u8,
                pub alignment: u32,
                pub size: u32
            }
            use core::ffi::c_void;
            //see https://opensource.apple.com/source/objc4/objc4-680/runtime/objc-runtime-new.h.auto.html
            #[repr(C)]
            pub struct ClassRoT {
                pub flags: u32,
                //Think this is 40 for metaclasses, not sure for regular classes
                pub instance_start: u32,
                pub instance_size: u32,
                pub reserved: u32, //clang emits .space	4
                //Usually 0, although I've seen 1 when the ivar is an `id`.
                pub ivar_layout: *const c_void, //.quad 0
                pub name: *const u8,
                pub base_method_list: *const c_void, //MethodListT
                pub base_protocols: *const c_void,
                pub ivars: *const IvarListT,
                pub weak_ivar_layout: *const c_void,
                pub base_properties: *const c_void,
            }
            //declare RO_FLAGS options
            pub const RO_FLAGS_METACLASS: u32 = 1;
            pub const RO_FLAGS_HIDDEN:u32 = 1<<4;
            pub const RO_FLAGS_ARR:u32 = 1<<7;

            pub const CLASS_FLAGS: u32 =RO_FLAGS_HIDDEN | RO_FLAGS_ARR;

            pub const METACLASS_FLAGS: u32 =RO_FLAGS_METACLASS | RO_FLAGS_HIDDEN | RO_FLAGS_ARR;

            //note: Class RoT instance needs to wait for ivar configuration
            //it cannot appear in the prelude.

            //However we can declare class type
            #[repr(C)]
            pub struct CLASST {
                //points to metaclass
                pub isa: *const *const c_void,
                pub superclass: *const *const c_void,
                // needs to be populated with extern OBJC_EMPTY_CACHE symbol
                pub cache:  *const *const c_void,
                pub vtable: *const c_void,
                pub ro: *const ClassRoT
            }
            //And some external symbols (only relies on $superclass)
            #[link(name="CoreFoundation",kind="framework")]
            extern {
                #[link_name="OBJC_METACLASS_$_NSObject"]
                pub static NSOBJECT_METACLASS: *const c_void;

                //In addition to that, we likely want symbols for whatever
                //our superclass is, if distinct
                //Some foundation types are abstract and therefore tricky to subclass
                objr::bindings::__static_extern!("OBJC_CLASS_$_",$superclass,
                    pub static $NSSUPER_CLASS: *const c_void;
                );
                objr::bindings::__static_extern!("OBJC_METACLASS_$_",$superclass,
                    pub static NSSUPER_METACLASS: *const c_void;
                );
            }
            #[link(name="objc",kind="dylib")]
            extern {
                #[link_name="_objc_empty_cache"]
                pub static $OBJC_EMPTY_CACHE: *const c_void;
            }
            objr::bindings::__static_asciiz!("__TEXT,__objc_classname,cstring_literals",pub $CLASS_NAME,$objcname);

            //declare metaclass RoT
            objr::bindings::__static_expr!("__DATA,__objc_const", "_OBJC_METACLASS_RO_$_",$objcname,
                static METACLASS_RO: objr::bindings::_SyncWrapper<ClassRoT> =
                objr::bindings::_SyncWrapper(ClassRoT {
                    flags: METACLASS_FLAGS,
                    instance_start: 40,
                    instance_size: 40,
                    reserved:0,
                    ivar_layout: std::ptr::null(),
                    name: &CLASS_NAME as *const u8,
                    base_method_list: std::ptr::null(),
                    base_protocols: std::ptr::null(),
                    ivars: std::ptr::null(),
                    weak_ivar_layout:std::ptr::null(),
                    base_properties: std::ptr::null(),
                });
            );

            //metaclass instance can go in prelude
            objr::bindings::__static_expr!("__DATA,__objc_data", "OBJC_METACLASS_$_",$objcname,
                pub static METACLASS: objr::bindings::_SyncWrapper<CLASST> = objr::bindings::_SyncWrapper(CLASST {
                    isa: unsafe{ &NSOBJECT_METACLASS},
                    superclass: unsafe{ &NSSUPER_METACLASS},
                    cache: unsafe{ &OBJC_EMPTY_CACHE},
                    vtable: std::ptr::null(),
                    ro: &METACLASS_RO.0
                });
            );
        });
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __objc_subclass_implpart_class_ro {
    ($objcname:ident,
        $payload:ty,$CLASS_NAME:expr,$IVARLISTEXPR:expr,$METHODLISTEXPR:expr) => {
        objr::bindings::__mod!(class_ro_,$objcname, {
            type ClassRoT = objr::bindings::__concat_3_idents!("super::subclass_impl_",$objcname,"::ClassRoT");
            objr::bindings::__static_expr!("__DATA,__objc_const", "_OBJC_CLASS_RO_$_",$objcname,
                pub static CLASS_RO: objr::bindings::_SyncWrapper<ClassRoT> = objr::bindings::_SyncWrapper(ClassRoT {
                    flags: objr::bindings::__concat_3_idents!("super::subclass_impl_",$objcname,"::CLASS_FLAGS"),
                    //not sure where these come from
                    instance_start: 8,
                    //8 plus whatever the size of our payload is
                    instance_size: 8 + std::mem::size_of::<$payload>() as u32,
                    reserved:0,
                    ivar_layout: std::ptr::null(),
                    name: &objr::bindings::__concat_3_idents!("super::subclass_impl_",$objcname,"::CLASS_NAME") as *const u8,
                    //In the case that we have methods, we want this to be the method list
                    base_method_list: $METHODLISTEXPR,
                    base_protocols: std::ptr::null(),
                    //in the case that we have ivars, we need a ptr to ivar layout here
                    ivars: $IVARLISTEXPR,
                    weak_ivar_layout: std::ptr::null(),
                    base_properties: std::ptr::null(),
                });
            );
        });

    }
}

///Declares a method list
#[macro_export]
#[doc(hidden)]
macro_rules! __objc_subclass_implpart_method_list {
    (
        $objcname:ident,
        [$($objcmethod: literal, $methodfn: expr),+],
        $METHOD_LIST:ident
    ) => {
        //method prelude
                //declare idents inside the prelude
                objr::__objc_sublcass_implpart_method_prelude!(MethodT,MethodListT);

                $(
                    objr::bindings::__static_asciiz_ident_as_selector!("__TEXT,__objc_methname,cstring_literals","METHNAME_",$methodfn,$objcmethod);
                    /*todo: The real objc compiler deduplicates these values across different functions.
                    I'm unclear on exactly what the value of deduplicating this is.  From studying compiled binaries
                    it appears that the *linker* also deduplicates local (`L`) symbols of this type, so I'm
                    uncertain if deduplicating this at the compile phase has any effect really.

                    Leaving this for now.
                    */
                    objr::bindings::__static_asciiz_ident_as_type_encoding!("__TEXT,__objc_methtype,cstring_literals","METHTYPE_",$methodfn,$objcmethod);
                )+

                const COUNT: usize = objr::bindings::__count!($($methodfn),*);
                objr::bindings::__static_expr!("__DATA,__objc_const","_OBJC_$_INSTANCE_METHODS_",$objcname,
                    static $METHOD_LIST: objr::bindings::_SyncWrapper<MethodListT<COUNT>> = objr::bindings::_SyncWrapper(
                        MethodListT {
                            magic: 24,
                            count: COUNT as u32,
                            methods: [
                                $(
                                    MethodT {
                                        name: & objr::bindings::__concat_idents!("METHNAME_",$methodfn) as *const u8,
                                        types: & objr::bindings::__concat_idents!("METHTYPE_",$methodfn) as *const u8,
                                        imp: $methodfn as *const c_void
                                    }
                                ),*
                            ]

                        }
                    );
                );
    }
}
///Declares an ivarlist (e.g., payload variants)
#[macro_export]
#[doc(hidden)]
macro_rules! __objc_subclass_implpart_ivar_list {
    ($objcname: ident, $payloadtype:ty, $FRAGILE_BASE_CLASS_OFFSET: ident, $IVAR_LIST:ident) => {
        objr::bindings::__static_asciiz!("__TEXT,__objc_methname,cstring_literals",IVAR_NAME,"payload");
            //don't explain to objc what type this is
            objr::bindings::__static_asciiz!("__TEXT,__objc_methtype,cstring_literals",IVAR_TYPE,"?");

            //This symbol seems involved in solving the fragile base class problem.
            //I am told that if the superclass changes its layout, this type.
            //will be updated to point to the new layout.
            //By default, we put this to 8 since we think our type starts at position 8
            //into the object?
            objr::bindings::__static_expr3!("__DATA,__objc_ivar", "OBJC_IVAR_$_",$objcname,".payload",
            static $FRAGILE_BASE_CLASS_OFFSET: u32 = 8;
            );
            type IvarListT = objr::bindings::__concat_3_idents!("subclass_impl_",$objcname,"::IvarListT");
            objr::bindings::__static_expr!("__DATA,__objc_const", "_OBJC_INSTANCE_VARIABLES_",$objcname,
                static $IVAR_LIST: objr::bindings::_SyncWrapper<IvarListT> = objr::bindings::_SyncWrapper(
                    IvarListT {
                        magic: 32,
                        count: 1,
                        offset: &FRAGILE_BASE_CLASS_OFFSET,
                        name: &IVAR_NAME as *const u8,
                    r#type: &IVAR_TYPE as *const u8,
                    alignment: std::mem::align_of::<$payloadtype>() as u32,
                    size: std::mem::size_of::<$payloadtype>() as u32,
                    }
                );
            );
    }
}
///This macro implements some methods on the wrapper type
///to access the underlying payload.
#[macro_export]
#[doc(hidden)]
macro_rules! __objc_subclass_impl_payload_access {
    ($pub:vis, $identifier:ident,$payload:ty, $FRAGILE_BASE_CLASS_OFFSET:ident) => {
        impl $identifier {
            /// Gets a mutable reference to the underlying payload.
            ///
            /// # Safety
            /// You must guarantee you are called from an exclusive, mutable context.
            ///
            /// # Design
            /// Similar to `UnsafeCell`, but
            /// 1.  Difficult to initialize a cell here
            /// 2.  I'm not sure if `UnsafeCell` is FFI-safe
            /// 3.  In practice, you need to initialize the objc memory close to 100% of the time to avoid UB.
            #[allow(dead_code)]
            $pub unsafe fn payload_mut(&self) -> &mut $payload {
                //convert to u8 to get byte offset
                let self_addr = (self as *const _ as *const u8);
                //offset by FRAGILE_BASE_CLASS
                //Note that a real objc compiler will optimize `FRAGILE_BASE_CLASS_OFFSET` to 8
                //when the superclass is known to be `NSObject` (e.g. the class is not fragile).
                //I am skipping that optimization for now.
                //todo: Maybe optimize this further

                //Note that we need to read_volatile here to get the real runtime payload,
                //not the payload known at compile time
                let payload_addr = self_addr.offset(std::ptr::read_volatile(&$FRAGILE_BASE_CLASS_OFFSET) as isize);

                let payload_typed_addr =std::mem::transmute(payload_addr);
                payload_typed_addr
            }
            #[allow(dead_code)]
            $pub fn payload(&self) -> &$payload {
                unsafe { self.payload_mut() } //coerce to non-mut
            }
        }
    }
}
#[macro_export]
#[doc(hidden)]
macro_rules! __objc_subclass_implpart_finalize {
    ($pub:vis,$identifier:ident,$objcname:ident,$superclass:ident,
    //these are imported into our scope
        $NSSUPER_CLASS:expr,$OBJC_EMPTY_CACHE:expr
    ) => {
        //declare class
        objr::bindings::__mod!(subclass_finalize_,$identifier, {
            type CLASST = objr::bindings::__concat_3_idents!("super::subclass_impl_",$identifier,"::CLASST");
            objr::bindings::__static_expr!("__DATA,__objc_data", "OBJC_CLASS_$_",$objcname,
                pub static CLASS: objr::bindings::_SyncWrapper<CLASST> = objr::bindings::_SyncWrapper(CLASST {
                    isa: unsafe{ std::mem::transmute(& objr::bindings::__concat_3_idents!("super::subclass_impl_", $identifier, "::METACLASS") )} ,
                    superclass: unsafe{ & objr::bindings::__concat_3_idents!("super::subclass_impl_", $identifier, "::NSSUPER_CLASS") },
                    cache: unsafe{ &objr::bindings::__concat_3_idents!("super::subclass_impl_", $identifier, "::OBJC_EMPTY_CACHE") },
                    vtable: std::ptr::null(),
                    ro: &objr::bindings::__concat_3_idents!("super::class_ro_",$objcname,"::CLASS_RO").0
                });
            );
        });


        use objr::bindings::{objc_instance};

        //declare our wrapper type
        //The declared type will be FFI-safe to an objc pointer, see documentation
        //for objc_instance!.
        objc_instance! {
            pub struct $identifier;
        }
        //We avoid using `objc_class!` macro here since it imports an external ObjC class.
        //As we are exporting a class, we provide our own conformance.
        //Should be safe because we're declaring the type
        impl objr::bindings::ObjcClass for $identifier {
            #[inline] fn class() -> &'static ::objr::bindings::Class<Self> {
                unsafe{ &*(&(objr::bindings::__concat_3_idents!("subclass_finalize_",$identifier,"::CLASS")).0 as *const _ as *const ::objr::bindings::Class<Self>) }
            }
        }
    }
}

///Emits the subclass impl in the case have a payload
#[macro_export]
#[doc(hidden)]
macro_rules! __objc_subclass_impl_with_payload_no_methods {
    (
    $pub:vis,$identifier:ident,$objcname:ident,$superclass:ident,$payload:ty
    ) => {
        objr::__objc_subclass_implpart_a!($pub,$identifier,$objcname,$superclass,
        //declare these identifiers into our local scope
        CLASS_NAME,NSSUPER_CLASS,OBJC_EMPTY_CACHE);
        //payload variant requires an ivar list
        objr::__objc_subclass_implpart_ivar_list!($objcname,$payload,FRAGILE_BASE_CLASS_OFFSET, IVAR_LIST);

        objr::__objc_subclass_implpart_class_ro!($objcname,$payload,CLASS_NAME,&super::IVAR_LIST.0,
            std::ptr::null() //Since we have no methods, we pass null for METHODLISTEXPR
        );
        objr::__objc_subclass_implpart_finalize!($pub,$identifier,$objcname,$superclass,NSSUPER_CLASS,OBJC_EMPTY_CACHE);
        objr::__objc_subclass_impl_payload_access!($pub,$identifier,$payload,FRAGILE_BASE_CLASS_OFFSET);

    }
}
#[macro_export]
#[doc(hidden)]
macro_rules! __objc_subclass_impl_no_payload_no_methods {
    ($pub:vis,$identifier:ident,$objcname:ident,$superclass:ident) => {
                objr::__objc_subclass_implpart_a!($pub,$identifier,$objcname,$superclass,
        //declare these identifiers into our local scope
        CLASS_NAME,NSSUPER_CLASS,OBJC_EMPTY_CACHE);

                objr::__objc_subclass_implpart_class_ro!($objcname,
                (), //for the no-payload case, use an empty type
                CLASS_NAME,
                //IVAREXPRESSION: use the null pointer since we have no payload
                    std::ptr::null(),
                //METHLISTEXPRESSION: Use the null pointer since we have no methods
                    std::ptr::null()
                );
                objr::__objc_subclass_implpart_finalize!($pub,$identifier,$objcname,$superclass,NSSUPER_CLASS,OBJC_EMPTY_CACHE);
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __objc_subclass_impl_no_payload_with_methods {
    ($pub:vis,$identifier:ident,$objcname:ident,$superclass:ident,
    [ $($objcmethod:literal => $methodfn:expr $(,)* )+ ]
    ) => {

                objr::__objc_subclass_implpart_a!($pub,$identifier,$objcname,$superclass,
                //declare these identifiers into our local scope
                CLASS_NAME,NSSUPER_CLASS,OBJC_EMPTY_CACHE);

                objr::__objc_subclass_implpart_method_list!( $objcname, [$($objcmethod, $methodfn),*], METHOD_LIST);

                objr::__objc_subclass_implpart_class_ro!($objcname,
                (), //for the no-payload case, use an empty type
                CLASS_NAME,
                //use the null pointer for our ivar expression since we have no payload
                    std::ptr::null(),
                //transmute our method_list into c_void
                    unsafe{ std::mem::transmute(&super::METHOD_LIST.0) }
                );
                objr::__objc_subclass_implpart_finalize!($pub,$identifier,$objcname,$superclass,NSSUPER_CLASS,OBJC_EMPTY_CACHE);
    }
}

///Variant with payload and methods
#[macro_export]
#[doc(hidden)]

macro_rules! __objc_subclass_impl_with_payload_with_methods {
($pub: vis, $identifier:ident,$objcname:ident,$superclass:ident,$payload:ty, [$($objcmethod:literal => $methodfn:expr $(,)* )+ ]) =>
    {
        objr::__objc_subclass_implpart_a!($pub,$identifier,$objcname,$superclass,
                //declare these identifiers into our local scope
                CLASS_NAME,NSSUPER_CLASS,OBJC_EMPTY_CACHE);
        //variant with payload
        objr::__objc_subclass_implpart_ivar_list!($objcname,$payload,FRAGILE_BASE_CLASS_OFFSET, IVAR_LIST);
        //variant with methods
        objr::__objc_subclass_implpart_method_list!( $objcname, [$($objcmethod, $methodfn),* ], METHOD_LIST);
        objr::__objc_subclass_implpart_class_ro!($objcname,
        $payload,
        CLASS_NAME,
        unsafe {std::mem::transmute(&super::IVAR_LIST.0)},
        unsafe{ std::mem::transmute(&super::METHOD_LIST.0) }
        );
        objr::__objc_subclass_implpart_finalize!($pub,$identifier,$objcname,$superclass,NSSUPER_CLASS,OBJC_EMPTY_CACHE);
        objr::__objc_subclass_impl_payload_access!($pub, $identifier,$payload,FRAGILE_BASE_CLASS_OFFSET);
    }
}


//subclass "real" implementation here
///Declares an objc subclass.
/// ```rust
/// use objr::objc_subclass;
/// objc_subclass! {
///     //Declare a Rust type named `Example`, which maps to the underlying objc class
///     pub struct Example {
///         //In the ObjC runtime, our type will be named `Example`
///         @class(Example)
///         //And will have `NSNull` as its superclass
///         @superclass(NSNull)
///         //Do not allocate any ivar storage for the class
///         payload: (),
///         methods: []
///     }
/// }
/// ```
///
/// # Methods
///
/// To declare a method on the subclass, use a syntax like
/// ```ignore
/// methods = [
///             "-(void) mySelector" => unsafe myRustFunction
/// ]
/// ```
///
/// Where the left part is an ObjC declaration and the right part is a Rust function.  Couple of notes:
///
/// 1.  Rust function must be `extern "C"`.  Failing to do this is UB.
/// 2.  The first two arguments to the Rust function are the pointer to Self, and the selector.
///     (arguments that are repr-transparent to these are OK as well).
/// 3.  All arguments and return values must be FFI-safe.
///
/// Here's a simple example
/// ```
/// use objr::bindings::*;
/// extern "C" fn example(objcSelf: Example, //repr-transparent to the pointer type
///                     sel: Sel) {
///     println!("Hello from rustdoc!");
/// }
/// objc_subclass! {
///     pub struct Example {
///         @class(Example)
///         @superclass(NSObject)
///         payload: (),
///         methods: [ "-(void) example" => unsafe example ]
///     }
/// }
/// ```
///
/// ## Returning values
///
/// In general, if you're implementing a method of +1 (that is, retain/strong) convention, you need to return a retained value.
/// This means you must use [std::mem::forget] on a StrongCell.
///
/// Alternatively, if you're implementing a method of +0 (that is, autorelease) convention, you need to return an autoreleased value.
/// While you can create an [objr::bindings::AutoreleasedCell] yourself, the best strategy is usually to return [objr::bindings::StrongCell::return_autoreleased()].
///
/// ## Dealloc
///
/// You can supply an implementation of dealloc in order to roll your own 'drop' behavior.
///
/// Note that unlike "modern ARC" objc, you must chain to `[super dealloc]`.
///
/// ### `.cxx_destruct`
///
/// A real objc compiler uses a different strategy for the compiler generated deinitializer than `deinit`.  When
/// the you create an objc class with `id` (e.g., strong) payloads, the compiler synthesizes a `.cxx_destruct`
/// selector and uses special runtime flags to indicate this selector should be called.  This allows
/// compiler synthesis to co-exist with a user-written `deinit`.
///
/// This is not currently supported by the macro but may be added in the future.
///
/// ## Arguments
/// The first argument to your C function is a pointer to `self`, and the second argument is a selector-pointer.
/// You may use any memory-compatible types for these arguments in Rust.  For example, the self argument can be
/// * `*const c_void` or `*mut c_void`.
/// * `*const Example` or `*mut Example` (it's memory-compatible with the `*const c_void`).  Convenience functions are implemented
///   on the wrapper type so this may be the useful one.  Keep in mind that it's up to you to not mutate from an immutable context.
///   For more info, see [crate::bindings::objc_instance!#safety]
///
/// For the selector argument, typically you use `Sel`.  `*const c_void` and `*const c_char` are also allowed.
///
/// # Payloads
/// Your ObjC type may have its own storage, inside the object.  This obviates the need
/// to allocate any external storage or somehow map between Rust and ObjC memory.
///
/// Currently, a single field is supported.  However, this field can be a Rust struct.
/// Payloads may also be 0-sized, for example `()` may be used.
///
/// To specify a payload, you use one of the following "payload specifiers"
///
/// ## `()`
/// Indicates a zero-sized payload.
///
/// Note that there is a subtle difference between using the tokens `()` and specifying a payload of 0-size (ex, `unsafe ininitialized nondrop ()`).
/// In the former case, we emit no payload to objc.  In the latter case, we emit storage of 0 size.  The `()` syntax is preferred.
///
/// ## `unsafe uninitialized nondrop T`
///
/// Storage for type T will be created.  This is
/// * uninitialized.  It is UB to read this before initialization.  Presumably, you need to write an objc `init` method and ensure it is called.
///   If you somehow read this memory without initialization, this is UB.
/// * nondrop.  Drop will never be called on this type
/// * `unsafe`, no memory management is performed.
///
///
/// ```
/// use objr::bindings::*;
/// objc_subclass! {
///     //Declare a Rust type named `Example`, which maps to the underlying objc class
///     pub struct Example {
///         //In the ObjC runtime, our type will be named `Example`
///         @class(Example)
///         //And will have `NSNull` as its superclass
///         @superclass(NSNull)
///         //The following storage will be allocated.  See the payload section.
///         payload: unsafe uninitialized nondrop u8,
///         methods: ["-(id) init" => unsafe init]
///     }
/// }
///
///     extern "C" fn init(objcSelf: *mut Example, sel: Sel) -> *const Example {
///         let new_self: &Example = unsafe{ &*(Example::perform_super(objcSelf,  Sel::init(), &ActiveAutoreleasePool::assume_autoreleasepool(), ()))};
///         //initialize the payload to 5
///         *(unsafe{new_self.payload_mut()}) = 5;
///         //return self per objc convention
///         new_self
///     }
///```
/// ### Payload memory management
/// One thing to keep in mind is that in general, memory management is significantly
/// different in ObjC and most Rust patterns simply do not work.
///
/// Suppose you try to have a `struct Payload<'a> {&'a Type}` payload.  A few issues with this:
///
/// 1.  Currently, Rust does not understand that `Payload` is inside `Example`.  Therefore,
///     the borrowchecker does not check that `'a` is valid for the lifetime of `Example`.
///
/// 2.  Even if this worked, in practice ObjC types are usually donated to the runtime
///     either explicitly or implicitly.  The extent of this is not necessarily documented
///     by ObjC people.  For example, in `https://lapcatsoftware.com/articles/working-without-a-nib-part-12.html`
///     it's discussed that `NSWindow` effectively had its lifetime extended in an SDK
///     release, with little in the way of documentation (in fact, I can only find discussion
///     of it there).  In practice, this "just happens" in ObjC.
///
///     Therefore, your options are generally some combination of:
///
///     1.  Store `'static` data only
///     2.  Use `StrongCell` for ObjC types.  This is simlar to what ObjC does internally anyway.
///     3.  Use `Rc` or similar for Rust data.
///     4.  I'm not gonna be the safety police and tell you not to use raw pointers,
///         but you are on your own as far as the unbounded lifetimes of ObjC objects.
///
/// Keep in mind that for several of these, you need to implement your own dealloc that calls drop.
///
/// ### Coda on init
///
/// The payload is born in an uninitialized state, which means any use of it is undefined.  Obviously,
/// you need to init it in some initializer.
///
/// Less obviously, it is tricky to init it correctly.  For example, you assign to the payload, you may
/// drop the "prior" (uninitialized) value, which is UB.
///
/// In theory, [std::mem::MaybeUninit] would solve this – assuming you remember to wrap all your values (or the payload itself).
/// In practice however, [std::mem::MaybeUnint.assume_init()] requires moving the value outside the payload,
/// which cannot really be done in this case.  See `https://github.com/rust-lang/rust/issues/63568` for details.
///
/// The alternative is to write into your payload_mut with [std::ptr::write], which does not drop the uninitialized value.
///
#[macro_export]
macro_rules! objc_subclass {
    (
        $pub:vis struct $identifier:ident {
            @class($objcname:ident)
            @superclass($superclass:ident)
            payload: unsafe uninitialized nondrop $payload:ty,
            methods: []
        }
    ) => {
        objr::__objc_subclass_impl_with_payload_no_methods!($pub,$identifier,$objcname,$superclass,$payload);
    };
    (
        $pub:vis struct $identifier:ident {
            @class($objcname:ident)
            @superclass($superclass:ident)
            payload: (),
            methods: []
        }
    ) => {
        objr::__objc_subclass_impl_no_payload_no_methods!($pub,$identifier,$objcname,$superclass);
    };
        (
        $pub:vis struct $identifier:ident {
            @class($objcname:ident)
            @superclass($superclass:ident)
            payload: (),
            methods: [ $($objcmethod:literal => unsafe $methodfn:expr $(,)?)+ ]
        }
    ) => {
        objr::__objc_subclass_impl_no_payload_with_methods!($pub,$identifier,$objcname,$superclass,
            [ $($objcmethod => $methodfn )* ]
        );
    };
    (
        $pub:vis struct $identifier:ident {
            @class($objcname:ident)
            @superclass($superclass:ident)
            payload: unsafe uninitialized nondrop $payload:ty,
            methods: [ $($objcmethod:literal => unsafe $methodfn:expr $(,)?)+ ]
        }
    ) => {
        objr::__objc_subclass_impl_with_payload_with_methods!($pub,$identifier,$objcname,$superclass,$payload,
            [ $($objcmethod => $methodfn )* ]
        );
    };


}

#[cfg(test)]
mod test {
    mod example {
        use objr::bindings::*;
        objc_subclass! {
            pub struct Example {
             @class(Example)
             @superclass(NSObject)
             payload: (),
             methods: [
                 "-(id) init" => unsafe sample
             ]
         }
        }
        extern "C" fn sample(objc_self: &Example, _sel: Sel) -> *const Example {
            println!("init from rust");
            unsafe { Example::perform_super(objc_self.assume_nonmut_perform(), Sel::init(), &ActiveAutoreleasePool::assume_autoreleasepool(), ()) }
        }
    }
    mod example_payload_no_methods {
        use objr::bindings::*;
        objc_subclass! {
         pub struct ExamplePN {
             @class(ExamplePN)
             @superclass(NSObject)
             payload: unsafe uninitialized nondrop u8,
             methods: []
         }
        }
    }
    mod example_payload_methods {
        use objr::bindings::*;
        objc_subclass! {
         pub struct ExamplePayloadMethods {
             @class(ExamplePayloadMethods)
             @superclass(NSObject)
             payload: unsafe uninitialized nondrop u8,
             methods: [
                 "-(id) init" => unsafe sample
             ]
         }
        }
        extern "C" fn sample(objc_self: &ExamplePayloadMethods, _sel: Sel) -> *const ExamplePayloadMethods {
            let new_self: &ExamplePayloadMethods = unsafe{ &*ExamplePayloadMethods::perform_super(objc_self.assume_nonmut_perform(), Sel::init(), &ActiveAutoreleasePool::assume_autoreleasepool(), () ) };
            *(unsafe{new_self.payload_mut()}) = 5;
            new_self
        }
    }
    mod example_dealloc {
        pub static DEALLOC_COUNT: AtomicBool = AtomicBool::new(false);

        use objr::bindings::*;
        use std::sync::atomic::{AtomicBool, Ordering};
        objc_subclass! {
             pub struct ExampleDealloc {
                 @class(ExampleDealloc)
                 @superclass(NSObject)
                 payload: unsafe uninitialized nondrop u8,
                 methods: [
                     "-(void) dealloc" => unsafe dealloc
                 ]
             }
        }
        extern "C" fn dealloc(objc_self: &mut ExampleDealloc, _sel: Sel) {
            let _: () = unsafe{ ExampleDealloc::perform_super_primitive(objc_self, Sel::from_str("dealloc"), &ActiveAutoreleasePool::assume_autoreleasepool(), ())};
            DEALLOC_COUNT.store(true,Ordering::SeqCst);
        }
    }

    #[test] fn subclass() {
        use objr::bindings::*;

        let pool = unsafe{ AutoreleasePool::new() };
        let _ = example::Example::class().alloc_init(&pool);
    }
    #[test] fn subclass_dealloc() {
        use objr::bindings::*;
        use std::sync::atomic::Ordering;
        let pool = unsafe{ AutoreleasePool::new() };
        assert!(example_dealloc::DEALLOC_COUNT.load(Ordering::SeqCst) == false);
        let _ = example_dealloc::ExampleDealloc::class().alloc_init(&pool);
        //ex dropped here
        assert!(example_dealloc::DEALLOC_COUNT.load(Ordering::SeqCst) == true);

    }

    #[test] fn initialize_payload() {
        use objr::bindings::*;
        let pool = unsafe{ AutoreleasePool::new() };
        let ex = example_payload_methods::ExamplePayloadMethods::class().alloc_init(&pool);
        assert!(*ex.payload() == 5);
    }

    #[test] fn multiple_subclasses() {
        use objr::bindings::*;
        // objc_subclass! {
        //     struct A {
        //         @class(A)
        //         @superclass(NSObject)
        //         payload: (),
        //         methods: []
        //     }
        // }
        // objc_subclass! {
        //     struct B {
        //         @class(B)
        //         @superclass(NSObject)
        //         payload: (),
        //         methods: []
        //     }
        // }
    }

}





