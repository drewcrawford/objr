//! NSError implementation

use super::bindings::*;
objc_class! {
    pub struct NSError;
    pub trait NSErrorTrait {
        @class(NSError)
    }
    impl NSErrorTrait for Class {}
}

