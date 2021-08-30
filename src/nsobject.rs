//! Bindings for NSObject
//!
use objr::bindings::{ActiveAutoreleasePool,Sel};

use super::nsstring::NSString;


use super::objcinstance::ObjcInstance;
use super::performselector::PerformsSelector;
use super::bindings::*;


//If you fail to Link CoreFoundation, description cannot be found
#[link(name="CoreFoundation",kind="framework")]
//If you fail to link Foundation, linker will not understand where NSString symbols come from
#[link(name="Foundation",kind="framework")]
extern {}
objc_selector_group!(
        pub trait NSObjectSelectors {
            @selector("alloc")
            @selector("description")
            @selector("respondsToSelector:")
            @selector("init")
            @selector("conformsToProtocol:")
            @selector("dealloc")
            @selector("copy")
        }
        impl NSObjectSelectors for Sel {}
    );

///Trait for NSObject.  This will be autoimplemented by all [ObjcInstance].
///
/// This type provides bindings to common `NSObject` functions.
pub trait NSObjectTrait: Sized + ObjcInstance {
    fn description<'a>(&self, pool: &ActiveAutoreleasePool) -> StrongCell<NSString>;
    //objc_method_declaration!{autoreleased fn description() -> NSString; }
    fn responds_to_selector(&self, pool: &ActiveAutoreleasePool, sel: Sel) -> bool;

    fn copy(&self, pool: &ActiveAutoreleasePool) -> StrongCell<Self>;

    ///Calls `[instance init]`.;
    unsafe fn init(receiver: *mut *mut Self, pool: &ActiveAutoreleasePool);
    ///erases type to NSObject
    fn as_nsobject(&self) -> &NSObject;
}
//"description" will not work unless CoreFoundation is linked
impl<T: ObjcInstance> NSObjectTrait for T {
    fn description<'a>(&self, pool:  &ActiveAutoreleasePool) -> StrongCell<NSString> {
        unsafe {
            let raw = Self::perform_autorelease_to_retain(self.assume_nonmut_perform(), Sel::description(), pool, ((),));
            NSString::assume_nonnil(raw).assume_retained()
        }
    }
    fn responds_to_selector(&self, pool: &ActiveAutoreleasePool, sel: Sel) -> bool {
        unsafe {
            Self::perform_primitive(self.assume_nonmut_perform(), Sel::respondsToSelector_(), pool, (sel,))
        }
    }
    fn copy(&self, pool: &ActiveAutoreleasePool) -> StrongCell<Self> {
        unsafe {
            let r = Self::perform(self.assume_nonmut_perform(), Sel::copy(), pool, ());
            Self::assume_nonnil(r).assume_retained()
        }
    }
    ///Initializes the object by calling `[self init]`
    ///
    ///By objc convention, `init` may return a distinct pointer than the one that's passed in.
    /// For this reason, a mutable reference is required.
    unsafe fn init(receiver: *mut *mut Self, pool: &ActiveAutoreleasePool) {
        //init can return a distinct pointer
        //upcast return type to mutable since it matches the argument
        let ptr = (Self::perform(*receiver,Sel::init(), pool, ())) as *const T as *mut T;
        *receiver = ptr;
    }

    fn as_nsobject(&self) -> &NSObject {
        unsafe {&* (self as *const Self as *const NSObject)}
    }
}
objc_class! {
    pub struct NSObject {
        @class(NSObject)
    }
}
