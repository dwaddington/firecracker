extern crate bindgen;

use std::path::Path;

fn main() {
    let header_path = "./src/mss_client_api.h";

    println!("cargo:rerun-if-changed={}", header_path);

    if !Path::new(&header_path).is_file() {
        panic!("mss_client_api.h not found in current dirctory")
    }

    {
        // run bindgen
        let bindings = bindgen::Builder::default()
            .header(header_path)
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .derive_debug(true)
            .layout_tests(false)
            .whitelist_type("segment_pair_t")
            .whitelist_function("mss_.*")
            .generate()
            .expect("unable to generate bindings for mss_client_api.h");

        let out_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/mss_api.rs");
        bindings
            .write_to_file(out_path)
            .expect("bindgen couldn't write mss_api.rs");
    }
    println!(
        "cargo:rustc-link-search=native={}",
        env!("CARGO_MANIFEST_DIR")
    );
    println!("cargo:rustc-link-lib=dylib=mss-client");
}
