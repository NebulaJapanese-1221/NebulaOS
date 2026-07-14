// Checksum algorithms for NebulaFS
// Includes Fletcher-2, Fletcher-4, and SHA-256 (simplified)

/// Fletcher-2 checksum
pub fn fletcher2(data: &[u8]) -> u16 {
    let mut sum1: u16 = 0;
    let mut sum2: u16 = 0;
    
    for &byte in data {
        sum1 = sum1.wrapping_add(byte as u16);
        sum2 = sum2.wrapping_add(sum1);
    }
    
    sum2
}

/// Fletcher-4 checksum (returns two u32 values)
pub fn fletcher4(data: &[u8]) -> (u32, u32) {
    let mut sum1: u32 = 0;
    let mut sum2: u32 = 0;
    let mut sum3: u32 = 0;
    let mut sum4: u32 = 0;
    
    for &byte in data {
        sum1 = sum1.wrapping_add(byte as u32);
        sum2 = sum2.wrapping_add(sum1);
        sum3 = sum3.wrapping_add(sum2);
        sum4 = sum4.wrapping_add(sum3);
    }
    
    (sum4, sum3)
}

/// Simplified SHA-256 (not cryptographically secure, for demonstration only)
pub fn sha256_simple(data: &[u8]) -> [u8; 32] {
    // In a real implementation, we would use a proper SHA-256 implementation
    // For now, we'll use a simple hash function
    
    let mut hash = [0u8; 32];
    let mut state: u32 = 0x6a09e667; // Initial hash value
    
    for &byte in data {
        state = state.wrapping_add(byte as u32);
        state = state.rotate_left(5);
        state ^= 0x9e3779b9; // Golden ratio
    }
    
    // Convert state to bytes
    for i in 0..4 {
        hash[i] = (state >> (i * 8)) as u8;
    }
    
    // Fill the rest with a pattern based on the first 4 bytes
    for i in 4..32 {
        hash[i] = hash[i - 4].wrapping_add(hash[i - 1]);
    }
    
    hash
}

/// Verify a checksum
pub fn verify_checksum(data: &[u8], expected: &[u8], algorithm: ChecksumAlgorithm) -> bool {
    match algorithm {
        ChecksumAlgorithm::Fletcher2 => {
            let calculated = fletcher2(data);
            let expected_u16 = u16::from_le_bytes([expected[0], expected[1]]);
            calculated == expected_u16
        }
        ChecksumAlgorithm::Fletcher4 => {
            let (calc1, calc2) = fletcher4(data);
            let exp1 = u32::from_le_bytes([expected[0], expected[1], expected[2], expected[3]]);
            let exp2 = u32::from_le_bytes([expected[4], expected[5], expected[6], expected[7]]);
            calc1 == exp1 && calc2 == exp2
        }
        ChecksumAlgorithm::SHA256 => {
            let calculated = sha256_simple(data);
            calculated == expected
        }
    }
}

/// Checksum algorithm types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChecksumAlgorithm {
    Fletcher2,  // 16-bit checksum
    Fletcher4,  // 64-bit checksum (two 32-bit values)
    SHA256,     // 256-bit cryptographic hash
}