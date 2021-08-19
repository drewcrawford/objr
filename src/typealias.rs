//! These are typealiases to the types used in objc

use std::os::raw::{c_ulong};

#[cfg(target_pointer_width = "64")]
pub(crate) type NSUInteger = c_ulong;