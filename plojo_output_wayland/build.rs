use std::env;
use std::path::PathBuf;

use wayland_scanner::{generate_code, Side};

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    generate_code(
        "protocol/input-method-unstable-v2.xml",
        out_dir.join("input_method_unstable_v2.rs"),
        Side::Client,
    );
    generate_code(
        "protocol/text-input-unstable-v3.xml",
        out_dir.join("text_input_unstable_v3.rs"),
        Side::Client,
    );
    generate_code(
        "protocol/virtual-keyboard-unstable-v1.xml",
        out_dir.join("virtual_keyboard_unstable_v1.rs"),
        Side::Client,
    );
    println!("cargo:rerun-if-changed=protocol/input-method-unstable-v2.xml");
    println!("cargo:rerun-if-changed=protocol/text-input-unstable-v3.xml");
    println!("cargo:rerun-if-changed=protocol/virtual-keyboard-unstable-v1.xml");
    println!("cargo:rerun-if-changed=build.rs");
}
