//SPDX-License-Identifier: MIT OR Apache-2.0
//! NSError implementation

use std::fmt::{Formatter};
use super::bindings::*;

objr::class::objc_class_no_debug! {
    pub struct NSError {
        @class(NSError)
    }
}




//there is pretty much no situation where we want NSError to contain a raw pointer.
//We want it to have an error message.
impl std::fmt::Debug for NSError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self,f)
    }
}

impl std::error::Error for NSError {}
//pretty sure this is implied based on how swift `try` works
unsafe impl Send for NSError {}

pub trait ResultNSError<T> {
    ///A friendlier unwrap for [NSError] that prints the error if you encounter it.
    fn unwrap_nserror(self, pool: &ActiveAutoreleasePool) -> T;
}
impl<T> ResultNSError<T> for Result<T,AutoreleasedCell<'_, NSError>> {
    fn unwrap_nserror(self, pool: &ActiveAutoreleasePool) -> T {
        match self {
            Ok(t) => { t}
            Err(e) => {
                panic!("{}",e.description(pool))
            }
        }
    }
}

impl<T> ResultNSError<T> for Result<T,StrongCell<NSError>> {
    fn unwrap_nserror(self, pool: &ActiveAutoreleasePool) -> T {
        match self {
            Ok(t) => { t}
            Err(e) => {
                panic!("{}",e.description(pool))
            }
        }
    }
}

#[test] fn check_err() {
    //ensure cell types implement NSError
    fn assert_err<T: std::error::Error>(_t: &T) { }

    objc_selector_group! {
        pub trait NSErrorSelectors {
            @selector("initWithDomain:code:userInfo:")
        }
        impl NSErrorSelectors for Sel {}
    }



    autoreleasepool(|pool| {
        let err = unsafe {
            let alloc = NSError::class().alloc(pool);
            let raw = NSError::perform_autorelease_to_retain(alloc, Sel::initWithDomain_code_userInfo(), pool, (objc_nsstring!("TestErrorDomain").assume_nonmut_perform(),123 as i64, 0 as i64));
            NSError::assume_nonnil(raw).assume_retained()
        };
        assert_err(&err);
        let debug_value = format!("{:?}",err);
        assert!(debug_value.contains("TestErrorDomain"))
    })

}