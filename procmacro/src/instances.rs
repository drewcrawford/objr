///Returns an implementation of ObjcInstance for type
pub fn instance_impl(_type: &str) -> String{
    format!(r#"
    unsafe impl ::objr::bindings::ObjcInstance for {TYPE} {{
        }}
        impl std::fmt::Display for {TYPE} {{
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{
                use ::objr::foundation::NSObjectTrait;
                //this ought to be safe, since the object was allocated somehow and we had an autoreleasepool for that.
                let fake_pool = unsafe {{ ::objr::bindings::ActiveAutoreleasePool::assume_autoreleasepool() }};
                write!(f, "{{}}",self.description(&fake_pool).to_str(&fake_pool))
            }}
        }}
    "#,TYPE=_type)
}