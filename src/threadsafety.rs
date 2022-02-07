use crate::bindings::ObjcInstance;

/** Indicates some particular use of a type is implied to be threadsafe by ObjC convention.

This type is used in cases where we are modeling some Cocoa API guarantee where the details
do not map cleanly onto the Rust ecosystem.  An example might be this pattern

```objc
@interface MyObject
- (void)someMethodThatTakesAnIdentityBlock:(MyObject* (^nonnull)(MyObject *))blockName;
@end
```

Generally, we expect this to be imported/bound into Rust as
```
# struct MyObject {}
impl MyObject {
    #[allow(non_snake_case)]
    fn someMethodThatTakesAnIdentityBlock<F: Fn(&MyObject) -> &MyObject + Sync + 'static>(&self, blockName: F) {
    todo!()
    }
}
```

Let's imagine what the implementation of `someMethodThatTakesAnIdentityBlock` could be:

1.  It simply calls `blockName`, e.g. on the current thread and stack frame.  In this case, nothing needs to be threadsafe.
2.  It moves `blockName` to some new thread and stack frame, and it executes there.  In this case, we need constraints like `Sync` and `'static`
    on our closure.

Sometimes, the right answer is clear from the context.  Other times, ObjC does not indicate which one really happens.  Moreover, even
if it works one way today, it could work the other in the future.

Therefore, it's often best to be conservative: add constraints like `+ Sync + 'static` (or `+ Send + 'static` for `FnOnce`) on the closure.
However, this opens a new problem: can such a closure work with types like `&MyObject`?

Obviously it can: the API exists and is intended to be called.  The trouble is, in Rust when we add `+ Sync + 'static` to our closure,
we also need to add [Sync] to our ObjC type.  ...But **is** it sync?  Well it depends on whether the implementor did 1 or 2, if they did
1, then it isn't really [Sync] and we can't implement that.  Moreover, in ObjC it's common for some methods to be threadsafe and others not,
which doesn't model well into the Rust typesystem where an entire type is `Sync` or it isn't.

To solve this problem, we introduce a new wrapper type around particular *uses*.  By wrapping the type, we indicate this *use* of it
is threadsafe in some way, without explaining which way exactly.  The wrapper requires an unsafe operation to add the wrapper and
remove the wrapper, so you must guarantee for the lifetime of the wrapper you know what you're doing.

*/
#[derive(Debug)]
pub struct ImpliedSyncUse<T>(T);

impl<T> ImpliedSyncUse<T> {
    /**
    Creates a wrapper type that guarantees this use is [Sync].

    # Safety
    You must guarantee that, from the time of this function until [unwrap], all operations performed meet the guarantees of [Sync].
    */
    #[inline] pub const unsafe fn new(t: T) -> Self {
        Self(t)
    }
    /**
    Unwraps the wrapper back to the original type.

    # Safety
    You must guarantee that, from the time of [new] until now, all operations performed meet the guarantees of Sync.
    */
    #[inline] pub unsafe fn unwrap(self) -> T {
        self.0
    }
}
unsafe impl<T> Sync for ImpliedSyncUse<T> {}
unsafe impl<T> Send for ImpliedSyncUse<T> {}


