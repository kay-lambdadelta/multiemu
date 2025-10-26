#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

include!(concat!(env!("OUT_DIR"), "/iso3166.rs"));
include!(concat!(env!("OUT_DIR"), "/iso639.rs"));
