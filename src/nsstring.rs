//! Provides NSString
//!
use super::bindings::*;
use std::ffi::{CStr};
use super::objcinstance::{ObjcInstance};
use core::ffi::c_void;
use std::os::raw::c_long;

objc_class! {
	pub struct NSString;
	pub trait NSStringTrait {
		let group_name = "objr";
		@class(NSString)
	}
	impl NSStringTrait for AnyClass {}
}

objc_selector_group!(
	pub trait NSStringSelectors {
		let group_name="objr";
		@selector("UTF8String")
	}
	impl NSStringSelectors for Sel {}
);
type CFIndex = c_long; //signed long

//defined in CFStringBuiltInEncodings
#[allow(non_snake_case, non_upper_case_globals)]
const CFStringEncodingUTF8: CFIndex = 0x08000100;

#[repr(transparent)]
struct CFStringRef(*const c_void);

#[allow(non_snake_case)]
extern {
	fn CFStringCreateWithBytes(allocatorRef: *const c_void,bytes: *const u8, numBytes: CFIndex, encoding: CFIndex, isExternalRepresentation:bool) -> CFStringRef;
}

impl NSString {
	///A constant-time initializer for `NSString`, primarily used from the [objc_nsstring!()] macro.
	pub const fn from_guaranteed(marker: GuaranteedMarker<NSString>) -> Self {
		NSString(marker)
	}
	///Converts to a stringslice
	pub fn to_str(&self, pool: &ActiveAutoreleasePool) -> &str {
		unsafe {
			let str_pointer = self.marker().perform_inner_ptr(Sel::UTF8String(),pool, ());
			let msg = CStr::from_ptr(str_pointer);
			msg.to_str().unwrap()
		}
	}
	///This will create an NSString from the argument.  Internally foundation will
	/// copy the pointer to its own memory.
	pub fn from_str(_pool: &ActiveAutoreleasePool, str: &str) -> StrongCell<Self> {
		let bytes = str.as_bytes();
		use std::convert::TryInto;
		let bytes_len = bytes.len().try_into().unwrap();
		unsafe {
			let string_ref = CFStringCreateWithBytes(std::ptr::null(),
													bytes.as_ptr(), bytes_len,
													CFStringEncodingUTF8,
													false);
			//CFStringRef is toll-free bridged, meaning the inner pointer "is" an `NSString`
			//transmute works around mutability
			let marker = GuaranteedMarker::new_unchecked(std::mem::transmute(string_ref.0));
			//CFStringCreate returns +1
			marker.assuming_retained()
		}

	}
}


#[test] fn from_str() {
	let example = "example string here";
	autoreleasepool(|pool| {
		let nsstring = NSString::from_str(pool, example);
		assert_eq!(nsstring.to_str(pool), example);
	})
}

#[test] fn static_str() {
	autoreleasepool(|pool| {
		let test = objc_nsstring!("My example literal");
		let description = test.description(pool);
		assert_eq!(description.to_str(pool), "My example literal");
	})
}