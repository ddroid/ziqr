fn main() {
    // No gresource/blueprint compilation needed — UI is built programmatically
    println!("cargo::rerun-if-changed=build.rs");
}
