//! NSError implementation

use super::bindings::*;
objc_class! {
    pub struct NSError {
        @class(NSError)
    }
}

