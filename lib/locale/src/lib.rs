#![no_std]

extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
};

include!(concat!(env!("OUT_DIR"), "/iso3166.rs"));
include!(concat!(env!("OUT_DIR"), "/iso639.rs"));
