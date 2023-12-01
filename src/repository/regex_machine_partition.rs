use std::num::NonZeroUsize;
use std::sync::Arc;
use lru::LruCache;
use regex::Regex;

pub(crate) struct RegexMachinePartition {
    pub map: LruCache<String, Arc<Regex>>
}

impl RegexMachinePartition{
    pub fn new(regex_partition_capacity: u16) -> Self {
        Self {
            map: LruCache::new(NonZeroUsize::new(regex_partition_capacity as usize).unwrap())
        }
    }
}