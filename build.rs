use std::process::Command;
use walkdir::WalkDir;
use std::{io};


fn main() {
    // Compile the wasm.
    Command::new("wasm-pack")
        .args(&["build", "wasm/", "--release", "--target", "web"])
        .status()
        .unwrap();
    println!("cargo:rerun-if-changed=wasm/*");

    update_urls().expect("Error while updating URLs.");
}

// Update URLs when files change, to enable efficient caching. 
fn update_urls() -> Result<(), io::Error> {
    for entry in WalkDir::new("static")
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok()) {
        if entry.metadata().unwrap().is_file() {
            let f_name = entry.file_name().to_string_lossy();
            if let Some(f_path) = entry.path().to_string_lossy().strip_prefix("static/") {
                println!("{}", f_path);
            }
        }
    }

    Ok(())
}