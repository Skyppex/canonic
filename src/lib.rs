#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod builder;
mod packed_list;
mod parser;
pub mod path;
mod zip_greedy;

#[cfg(feature = "std")]
pub mod std_path;
