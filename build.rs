fn main() {
    let arch = std::env::var("ARCH").unwrap_or_else(|_| "all".to_string());

    if arch == "all" || arch == "x86" {
        println!("cargo:rustc-cfg=arch=\"x86\"");
    }
    if arch == "all" || arch == "x86_64" {
        println!("cargo:rustc-cfg=arch=\"x86_64\"");
    }

    println!("cargo:rerun-if-env-changed=ARCH");
}
