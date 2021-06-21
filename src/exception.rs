///! Support for objc exceptions.

use std::ffi::c_void;

///Declared in hard-exception.m and compiled with build.rs
extern "C" {
    fn hard_exception(call: extern "C" fn(*mut c_void), context: *mut c_void  );
}

extern "C" fn thunk_void<F: FnOnce()>(context: &mut Option<F>) -> *mut c_void {
    println!("Thunk_void");
    let f = context.take().unwrap();
    f();
    std::ptr::null_mut()
}
///This function catches an objc exception raised in the closure.
///
/// Return values are not supported, this is primarily intended to facilitate debugging.
pub fn try_unwrap_void<F: FnOnce()>(closure: F){
    println!("Try unwrap void");
    let thunk_fn = thunk_void::<F> as extern "C" fn(&mut Option<F>) -> *mut c_void;
    let mut closure_indirect = Some(closure);
    unsafe{ hard_exception(std::mem::transmute(thunk_fn), std::mem::transmute(&mut closure_indirect)) };
}


#[test] fn test_catch() {
    try_unwrap_void(|| {
        println!("Hello world");
    })
}


