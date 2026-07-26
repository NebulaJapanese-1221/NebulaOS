// DNS Resolver for NebulaOS
// Resolves hostnames to IP addresses

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

pub struct DNSResolver {
    cache: BTreeMap<String, u32>,
    // In a real implementation, we would have a way to query DNS servers
    // For now, we'll use a hardcoded cache
}

impl DNSResolver {
    pub fn new() -> Self {
        let mut cache = BTreeMap::new();
        cache.insert(String::from("example.com"), 0x01020304); // 1.2.3.4
        cache.insert(String::from("google.com"), 0x08080808); // 8.8.8.8
        
        DNSResolver {
            cache,
        }
    }
    
    pub fn resolve(&self, hostname: &str) -> Option<u32> {
        self.cache.get(hostname).copied()
    }
}
