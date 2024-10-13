//! Misc helper functions
use proc_macro::{TokenTree, TokenStream};
///Returns an error
pub fn error(error: &str) -> TokenStream {
    //For whatever reason we can't use `compile_error!` with a quote
    let safe_str = error.replace('"', "\\\"");
    format!("compile_error!(\"{}\")",safe_str).parse().unwrap()
}

///In some cases, procmacros may be given a type in a "group" wrapper (with a single child).
/// This appears to be the case when they are invoked by another macro.
///
/// I do not know why.
fn unbox_group(tree: TokenTree) -> TokenTree {
    match &tree {
        TokenTree::Group(g) => {
            let mut iter = g.stream().into_iter();
            let unboxed = match iter.next() {
                Some(u) => u,
                None => TokenTree::Group(g.to_owned())
            };
            //check if this is a single child or not
            match iter.next() {
                //additional child: do not unbox and return the original tree
                Some(_) => TokenTree::Group(g.to_owned()),
                //no additional child, unbox
                None => unboxed
            }
        }
        other => other.to_owned()
    }
}

#[derive(Debug)]
pub enum ParsedLiteral {
    RawLiteral(()),
    Literal(String)
}
impl ParsedLiteral {
    pub fn unwrap_literal(self) -> String {
        match self {
            ParsedLiteral::Literal(l) => l,
            ParsedLiteral::RawLiteral(_) => panic!("Can't use a raw literal")
        }
    }
}

///Parses the a literal string, unboxing from a group if needed.
///
/// If no literal can be parsed, returns `Err`
pub fn parse_literal_string<I: Iterator<Item=TokenTree>>(iterator: &mut I) -> Result<ParsedLiteral,String> {
    let next = match iterator.next() {
        Some(u) => u,
        None => { return Err("Nothing found.".to_string())}
    };
    let unboxed_next = unbox_group(next);
    match unboxed_next {
        TokenTree::Literal(s) if s.to_string().starts_with('"') => {

            let mut parsed_string = s.to_string();
            parsed_string.remove(parsed_string.len()-1);
            parsed_string.remove(0);

            Ok(
            ParsedLiteral::Literal(parsed_string)
            )
        },
        //parse raw strings like r#"test"#
        TokenTree::Literal(s) if s.to_string().starts_with("r#\"") => {
            //watch out for indexing in this one
            let mut parsed_string = s.to_string();
            //remove 2 from the tail `"#`
            parsed_string.remove(parsed_string.len()-1);
            parsed_string.remove(parsed_string.len()-1);

            //remove 3 chars from the head `r#"`
            parsed_string.remove(0);
            parsed_string.remove(0);
            parsed_string.remove(0);
            Ok(ParsedLiteral::RawLiteral(()))
        }
        other => {
            Err(format!("unexpected {:?}",other))
        }
    }
}

///Parses identifier, unboxing from a group if needed.
///
/// If no literal can be parsed, returns `Err`
pub fn parse_ident<I: Iterator<Item=TokenTree>>(iterator: &mut I) -> Result<String,String> {
    let next = match iterator.next() {
        Some(u) => u,
        None => { return Err("Nothing found.".to_string()) }
    };
    let unboxed_next = unbox_group(next);
    match unboxed_next {
        TokenTree::Ident(s)  => { Ok(s.to_string()) }
        other => {
            Err(format!("unexpected {:?}", other))
        }
    }
}

pub fn parse_type<I: Iterator<Item=TokenTree>>(iterator: &mut I) -> Result<String,String> {
    let next = match iterator.next() {
        Some(u) => u,
        None => { return Err("Nothing found.".to_string()) }
    };
    let unboxed_next = unbox_group(next);
    match unboxed_next {
        TokenTree::Ident(s)  => { Ok(s.to_string()) }
        TokenTree::Group(g) => {
            let mut iter = g.stream().into_iter();
            let mut parsed = String::new();
            loop {
                let next = match iter.next() {
                    Some(u) => u,
                    None => { break; }
                };
                match next {
                    TokenTree::Ident(s) => {
                        parsed.push_str(&s.to_string());
                    }
                    TokenTree::Punct(p) => {
                        parsed.push_str(&p.to_string());
                    }
                    TokenTree::Literal(l) => {
                        parsed.push_str(&l.to_string());
                    }
                    TokenTree::Group(g) => {
                        parsed.push_str(&g.to_string());
                    }
                }
            }
            if parsed == "" || parsed == "()".to_string() {
                Ok("()".to_string())
            }
            else {
                Err(format!("while parsing a type, group, {:?}", parsed))
            }
        }
        other => {
            Err(format!("unexpected {:?}", other))
        }
    }
}

///Tries to parse as an ident or a string
pub fn parse_ident_or_literal<I: Iterator<Item=TokenTree> + Clone>(iterator: &mut I) -> Result<String,String> {
    let iterator_backup = iterator.clone();
    match parse_literal_string(iterator) {
        Ok(ParsedLiteral::Literal(l))  => {Ok(l)}
        _ => {
            //retry as ident
            *iterator = iterator_backup;
            match parse_ident(iterator) {
                Ok(ident) => {Ok(ident)}
                Err(e) => {Err(e)}
            }
        }
    }
}

