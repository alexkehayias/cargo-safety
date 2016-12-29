#![feature(proc_macro)]

extern crate syntex_syntax;
extern crate syntex_errors;
extern crate git2;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod checks;
pub mod reports;
