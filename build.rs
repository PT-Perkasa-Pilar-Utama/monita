// rust-embed needs `web/dist` to exist at compile time. `make web` fills it; without that step the
// binary still builds, and every page answers 503 with the command to run.
fn main() {
    let dist = std::path::Path::new("web/dist");
    if !dist.exists() {
        std::fs::create_dir_all(dist).expect("create web/dist");
    }
    println!("cargo:rerun-if-changed=web/dist");
}
