#[derive(Debug, thiserror::Error)]
pub enum O3StorageError {
    #[error("Hardware requirements not met: {0}")]
    HardwareError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn hardware_check() -> Result<(), O3StorageError> {
    check_cpu_architecture()?;
    check_memory()?;
    check_storage()?;
    Ok(())
}

fn check_cpu_architecture() -> Result<(), O3StorageError> {
    #[cfg(not(target_arch = "aarch64"))]
    {
        return Err(O3StorageError::HardwareError(
            "This system requires ARM64 (aarch64) architecture".to_string()
        ));
    }

    #[cfg(target_arch = "aarch64")]
    {
        use std::fs;
        
        if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
            if !cpuinfo.contains("Cortex-A76") && !cpuinfo.contains("cortex-a76") {
                tracing::warn!("CPU may not be Cortex-A76, performance may be suboptimal");
            }
        }
    }

    tracing::info!("CPU architecture check passed");
    Ok(())
}

fn check_memory() -> Result<(), O3StorageError> {
    use std::fs;
    
    let meminfo = fs::read_to_string("/proc/meminfo")
        .map_err(|e| O3StorageError::HardwareError(format!("Cannot read memory info: {}", e)))?;

    let total_memory_kb = meminfo
        .lines()
        .find(|line| line.starts_with("MemTotal:"))
        .and_then(|line| {
            line.split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u64>().ok())
        })
        .ok_or_else(|| O3StorageError::HardwareError("Cannot parse memory info".to_string()))?;

    let total_memory_gb = total_memory_kb / 1024 / 1024;
    
    if total_memory_gb < 8 {
        return Err(O3StorageError::HardwareError(
            format!("Insufficient memory: {}GB < 8GB required", total_memory_gb)
        ));
    }

    tracing::info!("Memory check passed: {}GB available", total_memory_gb);
    Ok(())
}

fn check_storage() -> Result<(), O3StorageError> {
    use std::fs;
    
    let storage_path = "/var/lib/o3storage";
    
    match fs::create_dir_all(storage_path) {
        Ok(_) => {},
        Err(e) => {
            return Err(O3StorageError::HardwareError(
                format!("Cannot create storage directory {}: {}", storage_path, e)
            ));
        }
    }

    if let Ok(metadata) = fs::metadata(storage_path) {
        if !metadata.is_dir() {
            return Err(O3StorageError::HardwareError(
                format!("Storage path {} is not a directory", storage_path)
            ));
        }
    }

    tracing::info!("Storage check passed: {} is accessible", storage_path);
    Ok(())
}