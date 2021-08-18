use super::bindings::*;
use std::fmt::{Debug};
objc_class! {
    pub struct NSError;
    pub trait NSErrorTrait {
        @class(NSError)
    }
    impl NSErrorTrait for AnyClass {}
}

///Provides certain convenience behaviors around unwrapping e.g. result types
pub trait UnwrapsWithNSError {
    type ExpectedType;
    fn unwrap_nserror(self, pool: &ActiveAutoreleasePool) -> Self::ExpectedType;
}

impl<'a, R: Debug> UnwrapsWithNSError for Result<R, AutoreleasedCell<'a, NSError>> {
    type ExpectedType = R;

    fn unwrap_nserror(self, pool: &ActiveAutoreleasePool) -> R {
        if self.is_err() {
            panic!("{}",self.unwrap_err().description(pool).to_str(pool))
        }
        self.unwrap()
    }
}

