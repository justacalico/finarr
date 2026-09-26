use std::fs;
use std::path::Path;

fn main() {
    // include_dir! needs the folder to exist at compile time. When the
    // Flutter web build hasn't run yet, drop in a placeholder so `cargo
    // build`/`cargo test` still work.
    let dist = Path::new("frontend/dist");
    if !dist.join("index.html").exists() {
        fs::create_dir_all(dist).expect("create frontend/dist");
        fs::write(
            dist.join("index.html"),
            "<!doctype html><title>finarr</title><p>Web UI not built. Run scripts/build-flutter.sh.</p>",
        )
        .expect("write placeholder index.html");
    }
    println!("cargo:rerun-if-changed=frontend/dist");
}
