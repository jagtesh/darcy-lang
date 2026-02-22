use std::path::PathBuf;

pub fn darcy_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("darcy")
}

#[cfg(feature = "darcy-compiled")]
pub mod darcy_gen {
    #![allow(dead_code)]
    #![allow(unused_parens)]
    #![allow(clippy::redundant_pattern)]
    #![allow(non_shorthand_field_patterns)]
    #![allow(unused_braces)]
    include!(concat!(env!("OUT_DIR"), "/darcy_gen.rs"));
}

#[cfg(feature = "darcy-compiled")]
pub use darcy_gen::*;
