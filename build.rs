use std::process::Command;

fn main() {
    Command::new("wasm-pack")
        .args(&["build", "wasm/", "--release", "--target", "web"])
        .status()
        .unwrap();
    println!("cargo:rerun-if-changed=wasm/*");
}
