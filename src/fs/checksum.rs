// Checksum algorithms for NebulaFS
// Includes Fletcher-2, Fletcher-4, and SHA-256 implementations

/// Checksum algorithm types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChecksumAlgorithm {
    Fletcher2,
    Fletcher4,
    SHA256,
}

/// Calculate Fletcher-2 checksum
pub fn fletcher2(data: &[u8]) -> u16 {
    let mut sum1: u16 = 0;
    let mut sum2: u16 = 0;
    
    for byte in data {
        sum1 = sum1.wrapping_add(*byte as u16);
        sum2 = sum2.wrapping_add(sum1);
    }
    
    sum2
}

/// Calculate Fletcher-4 checksum
/// Returns (sum1, sum2) where each is a u32
pub fn fletcher4(data: &[u8]) -> (u32, u32) {
    let mut sum1: u32 = 0;
    let mut sum2: u32 = 0;
    
    // Process data in 32-bit chunks for better performance
    let chunks = data.chunks_exact(4);
    let remainder = chunks.remainder();
    
    for chunk in chunks {
        let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        sum1 = sum1.wrapping_add(value);
        sum2 = sum2.wrapping_add(sum1);
    }
    
    // Process remaining bytes
    if !remainder.is_empty() {
        let mut value: u32 = 0;
        for (i, &byte) in remainder.iter().enumerate() {
            value |= (byte as u32) << (i * 8);
        }
        sum1 = sum1.wrapping_add(value);
        sum2 = sum2.wrapping_add(sum1);
    }
    
    (sum1, sum2)
}

/// Simplified SHA-256 implementation (not cryptographically secure)
/// For a real implementation, use a proper crypto library
pub fn sha256_simple(data: &[u8]) -> [u8; 32] {
    // This is a placeholder implementation
    // In a real filesystem, you would use a proper SHA-256 implementation
    let mut hash = [0u8; 32];
    
    // Simple hash for demonstration purposes
    for (i, &byte) in data.iter().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(byte).wrapping_mul((i + 1) as u8);
    }
    
    hash
}

/// Verify a checksum against data
pub fn verify_checksum(data: &[u8], checksum: &[u8], alg: ChecksumAlgorithm) -> bool {
    match alg {
        ChecksumAlgorithm::Fletcher2 => {
            if checksum.len() != 2 {
                return false;
            }
            let expected = u16::from_le_bytes([checksum[0], checksum[1]]);
            let calculated = fletcher2(data);
            expected == calculated
        }
        ChecksumAlgorithm::Fletcher4 => {
            if checksum.len() != 8 {
                return false;
            }
            let expected_sum1 = u32::from_le_bytes([checksum[0], checksum[1], checksum[2], checksum[3]]);
            let expected_sum2 = u32::from_le_bytes([checksum[4], checksum[5], checksum[6], checksum[7]]);
            let (calculated_sum1, calculated_sum2) = fletcher4(data);
            expected_sum1 == calculated_sum1 && expected_sum2 == calculated_sum2
        }
        ChecksumAlgorithm::SHA256 => {
            if checksum.len() != 32 {
                return false;
            }
            let calculated = sha256_simple(data);
            calculated == checksum
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fletcher2() {
        let data = b"Hello, World!";
        let checksum = fletcher2(data);
        assert_ne!(checksum, 0);
    }
    
    #[test]
    fn test_fletcher4() {
        let data = b"Hello, World!";
        let (sum1, sum2) = fletcher4(data);
        assert_ne!(sum1, 0);
        assert_ne!(sum2, 0);
    }
    
    #[test]
    fn test_sha256() {
        let data = b"Hello, World!";
        let hash = sha256_simple(data);
        assert_ne!(hash, [0u8; 32]);
    }
    
    #[test]
    fn test_verify_checksum() {
        let data = b"Test data";
        
        // Test Fletcher-2
        let checksum = fletcher2(data).to_le_bytes().to_vec();
        assert!(verify_checksum(data, &checksum, ChecksumAlgorithm::Fletcher2));
        
        // Test Fletcher-4
        let (sum1, sum2) = fletcher4(data);
        let mut checksum = sum1.to_le_bytes().to_vec();
        checksum.extend_from_slice(&sum2.to_le_bytes());
        assert!(verify_checksum(data, &checksum, ChecksumAlgorithm::Fletcher4));
        
        // Test SHA-256
        let checksum = sha256_simple(data).to_vec();
        assert!(verify_checksum(data, &checksum, ChecksumAlgorithm::SHA256));
    }
}