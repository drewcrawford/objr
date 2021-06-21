///Emits the function we need for a static string expression.
pub fn static_string(string_literal: &str) -> String {

    format!(r#"
    {{
		#[inline(never)] fn codegen_workaround() -> &'static objr::foundation::NSString {{
			/*
        Pretty much we want to emit some assembly like
    .section	__TEXT,__cstring,cstring_literals
    L_.str:                                 ## @.str
        .asciz	"find this static content"

        .section	__DATA,__cfstring
        .p2align	3                               ## @_unnamed_cfstring_
    L__unnamed_cfstring_:
        .quad	___CFConstantStringClassReference
        .long	1992                            ## 0x7c8
        .space	4
        .quad	L_.str
        .quad	24

        Specific lines are referenced in code below.

        Note that for whatever reason, rust really wants to emit .asciz directives,
        but the memory layout "should" be the same...
         */
			#[link_section = "__TEXT,__cstring,cstring_literals"]
			static STRING_LITERAL: [u8; {LITERAL_LENGTH}] = *b"{STRING_LITERAL}\0";
			extern {{
				#[link_name = "\x01___CFConstantStringClassReference"]
				static CFCONSTANT_STRING_CLASS_REFERENCE : *mut core::ffi::c_void;
			}}

			//Some kind of magic structure that can be casted to CFString directly
			//.p2align 3
			#[repr(C,packed(8))]
			struct CFStringStatic {{
				//.quad	___CFConstantStringClassReference
				constant_string_class_reference: objr::bindings::_SyncWrapper<&'static *mut core::ffi::c_void>,
				//.long	1992
				magic: u32,
				// .space	4
				space: [u8; 4],
				//.quad	L_.str
				str: objr::bindings::_SyncWrapper<*const u8>,
				//.quad	[len]
				magic_2: usize
			}}
			#[link_section = "__DATA,__cfstring"]
			static CFSTRING_REF: CFStringStatic = CFStringStatic {{
				constant_string_class_reference: unsafe{{ objr::bindings::_SyncWrapper(&CFCONSTANT_STRING_CLASS_REFERENCE)}},
				magic: 1992,
				space: [0; 4],
				str: objr::bindings::_SyncWrapper((&STRING_LITERAL) as *const u8),
				magic_2: {LITERAL_LENGTH_MINUS_ONE}
			}};
			static NSSTRING_EXPRESSION: objr::bindings::_SyncWrapper<objr::foundation::NSString> = unsafe{{ objr::bindings::_SyncWrapper(objr::foundation::NSString::from_guaranteed(objr::bindings::GuaranteedMarker::new_unchecked(std::mem::transmute(&CFSTRING_REF)))) }};
			&NSSTRING_EXPRESSION.0
		}}
		codegen_workaround()
	}}
    "#,STRING_LITERAL=string_literal,LITERAL_LENGTH=string_literal.len() + 1,LITERAL_LENGTH_MINUS_ONE=string_literal.len())
}