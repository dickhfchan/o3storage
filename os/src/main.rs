#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;
use bootloader_api::{entry_point, BootInfo};
use x86_64::VirtAddr;

mod allocator;
mod gdt;
mod interrupts;
mod memory;
mod serial;
mod vga_buffer;
mod keyboard;
mod filesystem;
mod network;
mod storage;
mod crypto;
mod scheduler;

use allocator::init_heap;
use memory::BootInfoFrameAllocator;
use storage::StorageManager;
use network::NetworkStack;
use crypto::CryptoEngine;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    println!("O3StorageOS v0.1.0 - Custom Rust OS for Maximum Security");
    println!("=========================================================");
    
    // Initialize core OS components
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    
    // Initialize memory management
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    
    init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
    
    println!("[OK] Memory management initialized");
    
    // Initialize hardware abstraction
    keyboard::init();
    serial::init();
    println!("[OK] Hardware interfaces initialized");
    
    // Initialize O3Storage components
    let crypto_engine = CryptoEngine::new();
    println!("[OK] Cryptographic engine initialized");
    
    let mut storage_manager = StorageManager::new();
    storage_manager.initialize().expect("Failed to initialize storage");
    println!("[OK] Custom storage system initialized");
    
    let mut network_stack = NetworkStack::new();
    network_stack.initialize().expect("Failed to initialize network");
    println!("[OK] Minimal network stack initialized");
    
    // Start the O3Storage service
    println!("[OK] Starting O3Storage distributed storage system...");
    start_o3storage_service(storage_manager, network_stack, crypto_engine);
    
    // Should never reach here
    println!("[ERROR] O3Storage service terminated unexpectedly");
    hlt_loop();
}

fn start_o3storage_service(
    storage: StorageManager, 
    network: NetworkStack, 
    crypto: CryptoEngine
) -> ! {
    use scheduler::Task;
    
    // Create O3Storage service tasks
    let storage_task = Task::new(move || {
        loop {
            storage.process_requests();
            scheduler::yield_now();
        }
    });
    
    let network_task = Task::new(move || {
        loop {
            network.process_packets();
            scheduler::yield_now();
        }
    });
    
    let consensus_task = Task::new(move || {
        loop {
            // Process consensus operations
            scheduler::yield_now();
        }
    });
    
    // Simple cooperative scheduler
    let mut scheduler = scheduler::Scheduler::new();
    scheduler.add_task(storage_task);
    scheduler.add_task(network_task);
    scheduler.add_task(consensus_task);
    
    scheduler.run()
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    exit_qemu(QemuExitCode::Success);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}