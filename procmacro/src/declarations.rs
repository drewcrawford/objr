//SPDX-License-Identifier: MIT OR Apache-2.0

//! Implements declaration parsing for objc headers
//!
//! Primarily used in subclassing.

#[derive(Debug)]
struct Type(String);
#[derive(Debug)]
struct SelectorPart(String);
#[derive(Debug)]
struct ArgumentName(String);

///Taken from https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtTypeEncodings.html#//apple_ref/doc/uid/TP40008048-CH100
#[derive(Debug)]
//ignore the fact that some of these cases are unused
#[allow(dead_code)]
enum ParsedType {
    Char,
    Int,
    Short,
    Long,
    LongLong,
    UChar,
    UInt,
    UShort,
    ULong,
    ULongLong,
    Float,
    Double,
    Bool,
    Void,
    CharStar,
    Object,
    Sel,
    //These types are included but may not be correctly parsed
    Array,
    Structure,
    Union,
    Bitfield,
    Pointer(Box<ParsedType>),
    Class,
    Unknown,
    //"Special" types, not part of the standard, but implemented for convenience
    CGRect,
    CGSize,
}

impl ParsedType {
    fn type_encoding(&self) -> String {
        //https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtTypeEncodings.html#//apple_ref/doc/uid/TP40008048-CH100
        match self {
            ParsedType::Char => "c".to_owned(),
            ParsedType::Int => "i".to_owned(),
            ParsedType::Short => "s".to_owned(),
            ParsedType::Long => "l".to_owned(),
            ParsedType::LongLong => "q".to_owned(),
            ParsedType::UChar => "C".to_owned(),
            ParsedType::UInt => "I".to_owned(),
            ParsedType::UShort => "S".to_owned(),
            ParsedType::ULong => "L".to_owned(),
            ParsedType::ULongLong => "Q".to_owned(),
            ParsedType::Float => "f".to_owned(),
            ParsedType::Double => "d".to_owned(),
            ParsedType::Bool => "B".to_owned(),
            ParsedType::Void => "v".to_owned(),
            ParsedType::CharStar => "*".to_owned(),
            ParsedType::Object => "@".to_owned(),
            ParsedType::Sel => ":".to_owned(),
            ParsedType::Array => "[v]".to_owned(), //treated as array to void
            ParsedType::Structure => "{n=v}".to_owned(), //treated as struct of void
            ParsedType::Union => "(n=v)".to_owned(), //union of void
            ParsedType::Bitfield => "b0".to_owned(), //0 bits
            ParsedType::Pointer(t) => {
                let mut s = "^".to_owned();
                s.push_str(&t.type_encoding());
                s
            }
            ParsedType::Class => "@".to_owned(),
            ParsedType::Unknown => "?".to_owned(),
            ParsedType::CGRect => "{CGRect={CGPoint=dd}{CGSize=dd}}".to_owned(),
            ParsedType::CGSize => "{CGSize=dd}".to_owned(),
        }
    }

    //This is declared as `Result` so we can make it a const fn.  However,
    //the err is basically a panic.
    const fn magic_size(&self) -> Result<u8,()> {

        /*On x64 anyway this appears to be the size of the type in bytes, rounded up to the nearest word.

        e.g. char is 1 byte, but we round up to 4
        int is 4 bytes and also rounded up to 4 etc.

        I assume this is some alignment or memory thing either part of C or objc, not sure which.

        Not handling the incomplete types since it seems like more work than it's worth.
         */
        match self {
            ParsedType::Char => Ok(4),
            ParsedType::Int => Ok(4),
            ParsedType::Short => Ok(4),
            ParsedType::Long => Ok(8),
            ParsedType::LongLong => Ok(8),
            ParsedType::UChar => Ok(4),
            ParsedType::UInt => Ok(4),
            ParsedType::UShort => Ok(4),
            ParsedType::ULong => Ok(8),
            ParsedType::ULongLong => Ok(8),
            ParsedType::Float => Ok(4),
            ParsedType::Double => Ok(8),
            ParsedType::Bool => Ok(4),
            ParsedType::Void => Err(()),
            ParsedType::CharStar => Ok(8),
            ParsedType::Object => Ok(8),
            ParsedType::Sel => Ok(8),
            ParsedType::Array => Err(()),
            ParsedType::Structure => Err(()),
            ParsedType::Union => Err(()),
            ParsedType::Bitfield => Err(()),
            ParsedType::Pointer(_) => Ok(8),
            ParsedType::Class => Ok(8),
            ParsedType::Unknown => Err(()),
            ParsedType::CGRect => Ok(32),
            ParsedType::CGSize => Ok(16),
        }
    }
    fn parse(str: &str) -> Self {
        match str {
            "CGSize" => ParsedType::CGSize,
            "NSSize" => ParsedType::CGSize,
            "CGRect" => ParsedType::CGRect,
            "NSRect" => ParsedType::CGRect,
            "id" => ParsedType::Object,
            "char" => ParsedType::Char,
            "int" => ParsedType::Int,
            "short" => ParsedType::Short,
            "long" => ParsedType::Long,
            "long long" => ParsedType::LongLong,
            "unsigned char" => ParsedType::UChar,
            "unsigned int" => ParsedType::UInt,
            "unsigned short" => ParsedType::UShort,
            "unsigned long"=>ParsedType::ULong,
            "unsigned long long"=>ParsedType::ULongLong,
            "float"=>ParsedType::Float,
            "double"=>ParsedType::Double,
            "bool"=>ParsedType::Bool,
            "BOOL"=>ParsedType::Bool,
            "void"=>ParsedType::Void,
            "char*"=>ParsedType::CharStar,
            "char *"=>ParsedType::CharStar,
            str if str.ends_with("*") => {
                //parse the part before the pointer
                let prior_to_ptr = str.split_at(str.len() - 2);
                let final_result = match ParsedType::parse(prior_to_ptr.0) {
                    ParsedType::Unknown => ParsedType::Object,
                    //valid types are pointers to the type
                    other => ParsedType::Pointer(Box::new(other))
                };
                final_result
            },
            "SEL"=>ParsedType::Sel,
            _ => ParsedType::Unknown
        }
    }
}

///This parses expressions such as `[-/+](ReturnType) selectorPart:(ArgumentType) ArgumentName`
#[derive(Debug)]
enum DeclarationParserState {
    Initial,
    ///e.g., `-(void)` is `void`
    ReturnType(Type),
    SelectorPart(SelectorPart),
    ArgumentType(Type),
    ArgumentName(ArgumentName) //loops back to SelectorPart
}

struct PartialDeclaration {
    selector_part: SelectorPart,
    argument_type: Type,
}

enum PartType {
    LoneSelector(SelectorPart), //something like `-(void) a`;
    Argument(PartialDeclaration) //something like `-(void) a:(int) foo`
}


struct ParsedDeclaration {
    //todo: methodkind
    return_type: Type,
    //All methods are required to have at least 1 part.
    //To model this in the typesystem, we store the first part inline
    first_part: PartType,
    //subsequent parts go in here
    next_parts: Vec<PartialDeclaration>

}

impl ParsedDeclaration {
    fn selector(&self) -> String {
        let mut s = String::new();
        match &self.first_part {
            PartType::LoneSelector(sel) => {
                s.push_str(&sel.0);
            }
            PartType::Argument(a) => {
                s.push_str(&a.selector_part.0);
                s.push(':');
            }
        }
        for part in &self.next_parts {
            s.push_str(&part.selector_part.0);
            s.push(':')
        }
        s
    }

    fn type_str(&self) -> String {
        let mut user_args = Vec::new();
        match &self.first_part {
            PartType::LoneSelector(_) => {}
            PartType::Argument(arg) => {
                user_args.push(ParsedType::parse(&arg.argument_type.0));
            }
        }
        for arg in &self.next_parts {
            user_args.push(ParsedType::parse(&arg.argument_type.0));
        }
        let return_type = ParsedType::parse(&self.return_type.0);
        //output starts with return type
        let mut output = return_type.type_encoding();
        //Next phrase is the entire size
        //calculate the arg size
        let user_arg_size = user_args.iter().fold(0, |a, b| a + b.magic_size().expect("magic_size"));
        let entire_size = user_arg_size
            + ParsedType::Object.magic_size().expect("magic_size") //implicit self arg
        + ParsedType::Sel.magic_size().expect("magic_size"); //implicit SEL arg
        //return type seems not to be included in this value.

        //this consists of
        //0.  entire_size
        //1. @0 => seems to indicate the self arg goes into some 0 slot
        //2. :{} => sel goes into slot 8
        output.push_str(&format!("{}@0:{}",entire_size,ParsedType::Object.magic_size().expect("magic_size")));

        let mut slot = ParsedType::Object.magic_size().expect("magic_size") + ParsedType::Sel.magic_size().expect("magic_size");
        for arg in user_args {
            output.push_str(&arg.type_encoding());
            output.push_str(&format!("{}",slot));
            //advance slot for next time?
            slot += arg.magic_size().expect("magic_size");
        }
        output
    }
}

const DEBUG_PARSER: bool = false;

impl ParsedDeclaration {

    fn from_str(str: &str) -> Result<Self,String> {
        let mut state = DeclarationParserState::Initial;
        let mut string_iter = str.chars();
        let mut return_type = None;

        let mut current_partial_argument_type = None;
        let mut current_partial_selector_part = None;

        let mut parsed_partials = Vec::new();

        while let Some(char) = string_iter.next(){

            //I thought about parsing in wider blocks than by characters but I think
            //it would complicate the tokenization (whitespace removal) somewhat.
            match state { //state is moved here.  After this point we need to reassign it.
                DeclarationParserState::Initial => {
                    if char == ' ' {
                        state = DeclarationParserState::Initial; //continue
                    }
                    else if char == '-' {
                        state = DeclarationParserState::ReturnType(Type(String::with_capacity(10)));
                    }
                    else {
                        return Err(format!("expected `-``near {:?}",char));
                    }
                }
                DeclarationParserState::ReturnType(partial_type) => {
                    if char == ' ' && partial_type.0.len() == 0 {
                        //ignore leading space
                        state = DeclarationParserState::ReturnType(partial_type);
                    }
                    else if char == ' ' {
                        return Err("Expected return type near ' '".to_owned());
                    }
                    else if char == '(' {
                        //ignore
                        state = DeclarationParserState::ReturnType(partial_type);
                    }
                    else if char == ')' {
                        //section complete
                        if DEBUG_PARSER {
                            println!("Parsed return type {:?}",partial_type);
                        }
                        return_type = Some(partial_type);
                        state = DeclarationParserState::SelectorPart(SelectorPart(String::with_capacity(20)));
                    }
                    else if char == '(' || char == ' ' {
                        //ignore
                        state = DeclarationParserState::ReturnType(partial_type);
                    }
                    else {
                        //extend type
                        let mut extended_type = partial_type.0;
                        extended_type.push(char);
                        state = DeclarationParserState::ReturnType(Type(extended_type));
                    }
                }
                DeclarationParserState::SelectorPart(partial_selector) => {
                    if char == ' ' && partial_selector.0.len() == 0 {
                        //ignore leading space
                        state = DeclarationParserState::SelectorPart(partial_selector);
                    }
                    else if char == ' ' {
                        return Err(format!("Expected `selector:` near {:?}", partial_selector))
                    }
                    else if char == ':' {
                        //section complete
                        if DEBUG_PARSER {
                            println!("Parsed {:?}",partial_selector);
                        }
                        current_partial_selector_part = Some(partial_selector);

                        state = DeclarationParserState::ArgumentType(Type(String::with_capacity(10)));
                    }
                    else {
                        //extend type
                        let mut partial_string = partial_selector.0;
                        partial_string.push(char);
                        state = DeclarationParserState::SelectorPart(SelectorPart(partial_string));
                    }
                }
                DeclarationParserState::ArgumentType(partial_type) => {
                    if char == ' ' && partial_type.0.len() == 0 {
                        //ignore leading whitespace
                        state = DeclarationParserState::ArgumentType(partial_type)
                    }
                    else if char == ' ' {
                        return Err(format!("Expected argument type near whitespace after {:?}",partial_type));
                    }
                    else if char == '(' { //ignore this token
                        state = DeclarationParserState::ArgumentType(partial_type)
                    }
                    else if char == ')' {
                        //section complete
                        if DEBUG_PARSER {
                            println!("Parsed argument type {:?}",partial_type);
                        }
                        current_partial_argument_type = Some(partial_type);
                        state = DeclarationParserState::ArgumentName(ArgumentName(String::with_capacity(10)));
                    }
                    else { //extend type
                        let mut new= partial_type.0;
                        new.push(char);
                        state = DeclarationParserState::ArgumentType(Type(new));
                    }
                }
                DeclarationParserState::ArgumentName(partial_name) => {
                    if char == ' ' && partial_name.0.len() == 0 {
                        //ignore leading whitespace
                        state = DeclarationParserState::ArgumentName(partial_name)
                    }
                    else if char == ' ' { //end of argument name
                        if DEBUG_PARSER {
                            println!("Parsed {:?}",partial_name);
                        }
                        let new_part = PartialDeclaration {
                            argument_type: current_partial_argument_type.take().unwrap(),
                            selector_part: current_partial_selector_part.take().unwrap()
                        };
                        parsed_partials.push(new_part);
                        state = DeclarationParserState::SelectorPart(SelectorPart(String::with_capacity(20)));
                    }
                    else {
                        let mut new = partial_name.0;
                        new.push(char);
                        state = DeclarationParserState::ArgumentName(ArgumentName(new));
                    }
                }
            }
        } //end of chars

        //at this point, the question is, did we stop at an OK location?
        let expected: Option<&'static str> = match state {
            DeclarationParserState::Initial => Some("-"),
            DeclarationParserState::ReturnType(_) => Some(")"),
            DeclarationParserState::SelectorPart(_) => None, //ok to stop here
            DeclarationParserState::ArgumentType(_) => Some(")"),
            DeclarationParserState::ArgumentName(_) => None, //ok to stop here
        };
        if let Some(expected) = expected {
            return Err(format!("Expected `{}` after {}",expected,str));
        }

        //Finish all our final states
        //If we were parsing an argument, finish the partial
        if let Some(t) = current_partial_argument_type.take() {
            parsed_partials.push(PartialDeclaration {
                argument_type: t,
                selector_part: current_partial_selector_part.take().expect("current_partial_selector_part")
            });
        }

        let first_part: PartType;
        match state {
            DeclarationParserState::SelectorPart(part) if parsed_partials.len() == 0 => {
                //In this case, we may have parsed a bit of a selector, but did not see a `:`
                //ex `-(void) foo;`
                //here we want this to be a lone selector
                first_part = PartType::LoneSelector(part);
            }
            _ => {
                //otherwise the first part is the removed first element
                first_part = PartType::Argument(parsed_partials.remove(0));
            }
        }


        Ok(ParsedDeclaration {
            return_type: return_type.expect("return_type"),
            first_part,
            next_parts: parsed_partials
        })
    }
}

///Uses the above typesystem to parse a declaration into a selector
pub fn parse_to_selector(declaration: &str) -> Result<String,String> {
    let decl = ParsedDeclaration::from_str(declaration);
    decl.map(|f| f.selector())
}


pub fn parse_to_type_encoding(declaration: &str) -> Result<String,String> {
    let decl = ParsedDeclaration::from_str(declaration);
    decl.map(|f| f.type_str())
}



#[test]
fn parse_declaration_1() {
    let parse_1 = ParsedDeclaration::from_str("-(void) bar");
    assert!(parse_1.is_ok());
    let t = parse_1.unwrap();
    assert_eq!(t.selector(), "bar");

    assert_eq!(t.type_str(), "v16@0:8");
}

#[test]
fn parse_declaration_2() {
    let parse_2 = ParsedDeclaration::from_str("-(void) a:(int) arg b: (float) arg2");
    assert!(parse_2.is_ok(),"{:?}",parse_2.err().unwrap());
    let p = parse_2.unwrap();
    assert_eq!(p.selector(), "a:b:");
    assert_eq!(p.type_str(), "v24@0:8i16f20");
}

#[test] fn parse_declaration_3() {
    let parse = ParsedDeclaration::from_str("-(id) initWithFrame:(CGRect) frame");
    assert!(parse.is_ok());
    let p = parse.unwrap();
    assert_eq!(p.selector(), "initWithFrame:");
    assert_eq!(p.type_str(), "@48@0:8{CGRect={CGPoint=dd}{CGSize=dd}}16");
}