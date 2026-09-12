use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let kernel_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let (arch, asm_file) = if target.contains("x86_64") {
        ("x86_64", kernel_dir.join("x86_64/start.asm"))
    } else {
        ("x86", kernel_dir.join("x86/start.asm"))
    };

    let obj_file = out_dir.join(format!("start_{}.o", arch));

    let nasm = if arch == "x86_64" {
        "nasm"
    } else {
        "nasm"
    };

    let mut cmd = Command::new(nasm);
    cmd.arg("-f")
        .arg(if arch == "x86_64" { "elf64" } else { "elf32" })
        .arg(&asm_file)
        .arg("-o")
        .arg(&obj_file);

    if !cmd.status().unwrap().success() {
        panic!("Failed to assemble {}", asm_file.display());
    }

    println!("cargo:rustc-link-arg={}", obj_file.display());
    println!("cargo:rerun-if-changed={}", asm_file.display());
}