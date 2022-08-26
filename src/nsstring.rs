//! Provides NSString
//!
use super::bindings::*;
use std::ffi::{CStr};
use std::hash::{Hash, Hasher};
use std::os::raw::{c_char};
use crate::objcinstance::NonNullImmutable;
use objr::typealias::NSUInteger;

objc_class! {
	pub struct NSString {
		@class (NSString)
	}
}

objc_selector_group!(
	pub trait NSStringSelectors {
		@selector("UTF8String")
		@selector("initWithBytes:length:encoding:")
		@selector("isEqualToString:")
		@selector("hash")
	}
	impl NSStringSelectors for Sel {}
);

#[allow(non_upper_case_globals)]
const NSUTF8StringEncoding: NSUInteger = 4;


impl PartialEq for NSString {
	fn eq(&self, other: &Self) -> bool {
		unsafe {
			//I am reasonably confident this doesn't allocate
			let pool = ActiveAutoreleasePool::assume_autoreleasepool();
			NSString::perform_primitive(self.assume_nonmut_perform(), Sel::isEqualToString_(),&pool, (other.assume_nonmut_perform(),) )
		}
	}
}
impl Eq for NSString {}
impl Hash for NSString {
	fn hash<H: Hasher>(&self, state: &mut H) {
		unsafe {
			//I am reasonably confident this doesn't allocate
			let pool = ActiveAutoreleasePool::assume_autoreleasepool();
			let hash: NSUInteger = NSString::perform_primitive(self.assume_nonmut_perform(), Sel::hash(),&pool, () );
			state.write_u64(hash);
		}
	}
}

impl NSString {
	///Converts to a stringslice
	pub fn to_str(&self, pool: &ActiveAutoreleasePool) -> &str {
		unsafe {
			let str_pointer: *const c_char = Self::perform_primitive(self.assume_nonmut_perform(), Sel::UTF8String(), pool, ());
			//todo: using utf8 directly might be faster as this involves an up-front strlen in practice
			let msg = CStr::from_ptr(str_pointer);
			msg.to_str().unwrap()
		}
	}
	///Copies the string into foundation storage
	pub fn with_str_copy(str: &str, pool: &ActiveAutoreleasePool) -> StrongMutCell<NSString> {
		unsafe {
			let instance = Self::class().alloc(pool);
			let bytes = str.as_bytes().as_ptr();
			let len = str.as_bytes().len() as NSUInteger;

			let instance: *const NSString = Self::perform(instance,Sel::initWithBytes_length_encoding(),pool, (bytes.assume_nonmut_perform(),len,NSUTF8StringEncoding));
			//although this method is technically nullable, the fact that the string is already statically known to be utf8
			//suggests we should be fine
			NonNullImmutable::assume_nonnil(instance).assume_retained().assume_mut()
		}
	}
}



#[test] fn from_str() {
	use crate::autorelease::AutoreleasePool;
	let example = "example string here";
	let pool = unsafe{ AutoreleasePool::new() };
	let nsstring = NSString::with_str_copy(example, &pool);
	assert_eq!(nsstring.to_str(&pool), example);
}

#[test] fn static_str() {
	use crate::autorelease::AutoreleasePool;
	let pool = unsafe{ AutoreleasePool::new() };

	let test = objc_nsstring!("My example literal");
	let description = test.description(&pool);
	assert_eq!(description.to_str(&pool), "My example literal");
}

#[test] fn hash_str() {
	use std::collections::hash_map::DefaultHasher;

	autoreleasepool(|pool| {
		let s1 = objc_nsstring!("example string goes here");
		let s2 = NSString::with_str_copy("example string goes here",pool);
		let s2_p: &NSString = &s2;
		assert_eq!(s1,s2_p);

		let mut hashstate = DefaultHasher::new();
		let mut hashstate2 = DefaultHasher::new();
		assert_eq!(s1.hash(&mut hashstate),s2_p.hash(&mut hashstate2));

		fn assert_cell<H: Hash>(_h: &H) {}
		assert_cell(s1);
		assert_cell(&s2);
		assert_cell(s2_p);
	});

}