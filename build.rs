fn main() {
    // Bake the CI release tag into the binary so the GUI can show it. Falls back
    // to "dev" for local builds where BUILD_TAG is not set.
    let tag = std::env::var("BUILD_TAG").unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=BUILD_TAG={tag}");
    println!("cargo:rerun-if-env-changed=BUILD_TAG");

    slint_build::compile("ui/main.slint").unwrap();
}
