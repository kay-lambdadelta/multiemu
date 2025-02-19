use std::path::PathBuf;

use multiemu_macros::platform_aliases;
use multiemu_wgsl_bindgen::ShaderOutputType;

fn main() {
    platform_aliases!();

    println!("cargo:rerun-if-changed=shader");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    multiemu_wgsl_bindgen::Builder::new("shader/egui.wgsl")
        .enable_shader_output(ShaderOutputType::Glsl, "3.3.0")
        .enable_shader_output(ShaderOutputType::Spirv, "1.0.0")
        .generate(out_dir.join("egui.rs"))
        .unwrap();
}
