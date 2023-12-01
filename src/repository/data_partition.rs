use std::collections::HashMap;
use std::sync::atomic::AtomicU32;

pub(crate) struct DataPartition {
    pub map: HashMap<String, (Vec<u8>, AtomicU32)>
}

impl DataPartition{
    pub fn new() -> Self {
        Self {
            map: HashMap::new()
        }
    }
}