//! This module implements 'flat parsing', e.g. doing a depth-first
//! traversal of groups.
//!
//! This is often useful because, for reasons I don't fully understand,
//! Rust likes to put things in random groups when I don't think they should
//! be, I just want to parse some literal or ident for example.
use proc_macro::{TokenTree,Ident};
use proc_macro::token_stream::IntoIter;
///Like TokenTree, but without [TokenTree::Group]
#[derive(Debug)]
pub enum FlatTree {
    Ident(Ident),
    Punct(()),
    Literal(())
}
///Iterator type for `FlatTree`.
pub struct FlatIterator {
    inner: Vec<IntoIter>
}
impl FlatIterator {
    ///Creates a new iterator from a `TokenStream` iterator.
    pub fn new(into_iter: IntoIter) -> FlatIterator {
        FlatIterator{ inner: vec!(into_iter) }
    }
}

impl Iterator for FlatIterator {
    type Item = FlatTree;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        loop {
            //pop uses last in Rust
            let next_iterator = self.inner.last_mut()?;
            match next_iterator.next() {
                None => {
                    self.inner.pop();
                    continue;
                }
                Some(tree) => {
                    match tree {
                        TokenTree::Group(g) => {
                            let new_iterator = g.stream().into_iter();
                            self.inner.push(new_iterator);
                            continue;
                        }
                        TokenTree::Ident(i) => {
                            return Some(FlatTree::Ident(i))
                        }
                        TokenTree::Punct(..) => {
                            return Some(FlatTree::Punct(()))
                        }
                        TokenTree::Literal(..) => {
                            return Some(FlatTree::Literal(()))
                        }
                    }
                }
            }
        }

    }
}

// fn scratch() {
//     let f = TokenTree::
// }