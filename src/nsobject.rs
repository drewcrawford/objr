//! Bindings for NSObject
//!
use objr::bindings::{ActiveAutoreleasePool,Sel};

use super::nsstring::NSString;


use super::objcinstance::ObjcInstance;
use super::performselector::PerformsSelector;
use super::bindings::*;

struct Foo(*const [u8; 12]);
unsafe impl Send for Foo {}
unsafe impl Sync for Foo {}

//If you fail to Link CoreFoundation, description cannot be found
#[link(name="CoreFoundation",kind="framework")]
//If you fail to link Foundation, linker will not understand where NSString symbols come from
#[link(name="Foundation",kind="framework")]
extern {}
objc_selector_group!(
        pub trait NSObjectSelectors {
            let group_name = "objr";
            @selector("alloc")
            @selector("description")
            @selector("respondsToSelector:")
            @selector("init")
            @selector("conformsToProtocol:")
            @selector("dealloc")
        }
        impl NSObjectSelectors for Sel {}
    );

///Trait for NSObject.  This will be autoimplemented by all [ObjcInstance].
///
/// This type provides bindings to common `NSObject` functions.
pub trait NSObjectTrait {
    fn description<'a>(&self, pool: &ActiveAutoreleasePool) -> StrongCell<NSString>;
    //objc_method_declaration!{autoreleased fn description() -> NSString; }
    fn responds_to_selector(&self, pool: &ActiveAutoreleasePool, sel: Sel) -> bool;

    ///Calls `[instance init]`.;
    unsafe fn init(&mut self, pool: &ActiveAutoreleasePool);
    unsafe fn conforms_to_protocol(&self, pool: &ActiveAutoreleasePool, protocol: *const std::ffi::c_void) -> bool;
}
//"description" will not work unless CoreFoundation is linked
impl<T: ObjcInstance> NSObjectTrait for T {
    fn description<'a>(&self, pool:  &ActiveAutoreleasePool) -> StrongCell<NSString> {
        unsafe { self.marker().perform_autorelease_to_strong_nonnull(Sel::description(), pool,((),)) }
    }
    fn responds_to_selector(&self, pool: &ActiveAutoreleasePool, sel: Sel) -> bool {
        unsafe { self.marker().perform_primitive( Sel::respondsToSelector_(),pool, (sel,)) }
    }
    //todo: get a real protocol signature
    unsafe fn conforms_to_protocol(&self, pool: &ActiveAutoreleasePool, protocol: *const core::ffi::c_void) -> bool {
        self.marker().perform_primitive(Sel::conformsToProtocol_(), pool, (protocol,))
    }
    ///Initializes the object by calling `[self init]`
    ///
    ///By objc convention, `init` may return a distinct pointer than the one that's passed in.
    /// For this reason, a mutable reference is required.
    unsafe fn init(&mut self, pool: &ActiveAutoreleasePool) {
        //init can return a distinct pointer, so we need to write back into the receiver's marker.
        //This occurs for certain foundation objects, e.g. I have seen it with `NSDate`.
        use crate::performselector::PerformsSelectorPrivate;
        let result: GuaranteedMarker<T> = self.marker().perform_unmanaged_nonnull(Sel::init(), pool, ());
        *self.marker_mut() = result;
    }
}
objc_class! {
    pub struct NSObject;
    pub trait NSObjectAnyClassTrait {
        let group_name = "objr";
        @class(NSObject)
    }
    impl NSObjectAnyTrait for AnyClass {}
}
