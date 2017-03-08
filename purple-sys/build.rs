extern crate bindgen;
extern crate pkg_config;

use std::path::Path;
use std::env;

fn main() {

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file = Path::new(&out_dir).join("purple.rs");
    let mut bindings = bindgen::builder()
        .no_unstable_rust()
        .bitfield_enum("PURPLE_ICON_SCALE_.*|OPT_PROTO_.*|PURPLE_MESSAGE_.*");

    let purple_lib = pkg_config::probe_library("purple").unwrap();

    let purple_header = purple_lib.include_paths[0].join("purple.h");
    let purple_header = purple_header.to_str().unwrap();

    println!("libpurple main header: {}", purple_header);
    // Set main header path
    bindings = bindings.header(purple_header);

    for include_path in purple_lib.include_paths {
        let include_path = include_path.to_str().unwrap();

        println!("Adding include dir: {}", include_path);

        // Add each required include dir provided by pkg-config
        bindings = bindings.clang_arg("-I")
            .clang_arg(include_path)
            .clang_arg("-D")
            .clang_arg("PURPLE_PLUGINS");
    }

    // bindings.forbid_unknown_types();
    bindings = bindings.emit_builtins();

    bindings = bindings.derive_debug(false);

    match bindings.generate() {
        Ok(bindings) => {
            bindings.write_to_file(out_file).unwrap();
        }
        _ => panic!("Bindings generation failed"),
    }
}
