//! [`PrettyPrint`](crate::pretty::PrettyPrint) implementations for the `syn`
//! syntax tree, so Rust code embedded in a macro body is laid out natively by
//! the pretty printer, comments included.
//!
//! The implementations print the tree in source order: every leaf token moves
//! the printer's cursor through its original span, which lets the printer
//! interleave the comments and blank lines captured by the trivia lexer.

mod attr;
mod common;
mod expr;
mod item;
mod mac;
mod pat;
mod path;
mod stmt;
mod token;
mod ty;
