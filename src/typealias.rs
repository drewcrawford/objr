//! These are typealiases to the types used in objc

use std::os::raw::{c_ulong,c_long};

#[cfg(target_pointer_width = "64")]
pub type NSUInteger = c_ulong;
#[cfg(target_pointer_width = "64")]
pub type NSInteger = c_long;