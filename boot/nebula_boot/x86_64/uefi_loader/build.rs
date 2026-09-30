fn main() {
    println!("cargo:rustc-link-arg=/entry:efi_main");
    println!("cargo:rustc-link-arg=/subsystem:efi_application");
}