//! NSError implementation

use super::bindings::*;
objc_class! {
    pub struct NSError {
        @class(NSError)
    }
}


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