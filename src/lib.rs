#![deny(
    absolute_paths_not_starting_with_crate,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    missing_copy_implementations,
    non_ascii_idents,
    nonstandard_style,
    noop_method_call,
    pointer_structural_match,
    private_in_public,
    rust_2018_idioms,
    unused_qualifications
)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

#[cfg(feature = "concurrent")]
mod concurrent;
mod singlethread;

#[cfg(feature = "concurrent")]
pub use crate::concurrent::*;

pub use crate::singlethread::*;
