// Build script for O3StorageOS

fn main() {
    // Tell Cargo to rebuild if any source files change
    println!("cargo:rerun-if-changed=src/");
    
    // Enable minimal runtime for maximum security
    println!("cargo:rustc-cfg=no_std");
    println!("cargo:rustc-cfg=no_main");
    
    // Target-specific optimizations for ARM64
    #[cfg(target_arch = "aarch64")]
    {
        println!("cargo:rustc-cfg=cortex_a76");
        println!("cargo:rustc-link-arg=-mcpu=cortex-a76");
    }
    
    // Security hardening flags
    println!("cargo:rustc-link-arg=-Wl,-z,relro,-z,now");
    println!("cargo:rustc-link-arg=-Wl,--strip-all");
    
    // Create bootable image
    println!("cargo:warning=Building O3StorageOS - Zero Dependency Operating System");
}