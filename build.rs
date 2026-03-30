use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Get the output directory provided by Cargo
    let out_dir = env::var_os("OUT_DIR").map(PathBuf::from).expect("OUT_DIR not set");
    let dest_path = out_dir.join("generated_symbols.rs");

    // Dynamically determine the path to the kernel binary based on current target and profile
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from).unwrap();
    let target = env::var("TARGET").unwrap_or_else(|_| "i686-unknown-none".to_string());
    // Cargo strips .json from custom target files for the directory name
    let target_dir_name = target.strip_suffix(".json").unwrap_or(&target);
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    
    let kernel_bin = manifest_dir
        .join("target")
        .join(target_dir_name)
        .join(profile)
        .join("nebula_os");

    let sym_file = manifest_dir.join("kernel.sym");

    let mut output = String::from("pub static KERNEL_SYMBOLS: &[KernelSymbol] = &[\n");
    let mut nm_content = None;

    // Priority 1: Automated extraction via 'nm' (or 'llvm-nm') if binary exists
    if kernel_bin.exists() {
        let nm_tools = ["nm", "llvm-nm", "i686-elf-nm"];
        for tool in nm_tools {
            if let Ok(res) = Command::new(tool).arg("-n").arg(&kernel_bin).output() {
                if res.status.success() {
                    nm_content = Some(String::from_utf8_lossy(&res.stdout).into_owned());
                    break;
                }
            }
        }
    }

    // Priority 2: Fallback to manual sym file if 'nm' failed or binary doesn't exist yet
    if nm_content.is_none() && sym_file.exists() {
        nm_content = fs::read_to_string(sym_file).ok();
    }

    if let Some(content) = nm_content {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let addr_str = parts[0];
                let type_char = parts[1];
                let name = parts[2];
                
                // Filter for text (code) symbols. 
                // 'T' is global text, 't' is local text.
                if type_char == "T" || type_char == "t" {
                    output.push_str(&format!("    KernelSymbol {{ addr: 0x{}, name: {:?} }},\n", addr_str, name));
                }
            }
        }
    }

    output.push_str("];\n");
    fs::write(dest_path, output).unwrap();

    // Re-run the script if the binary, build script, or manual sym file changes
    println!("cargo:rerun-if-changed=build.rs");
    if kernel_bin.exists() {
        println!("cargo:rerun-if-changed={}", kernel_bin.display());
    }
    println!("cargo:rerun-if-changed=kernel.sym");
}