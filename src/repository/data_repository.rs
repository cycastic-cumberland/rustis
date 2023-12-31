use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem::size_of;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use base64::Engine;
use regex::Regex;
use tokio::sync::RwLock;
use crate::repository::data_partition::DataPartition;
use crate::repository::regex_machine_partition::RegexMachinePartition;

const DEFAULT_PARTITION_COUNT: u16 = 16;

pub struct DataRepository {
    data_partitions: Vec<RwLock<DataPartition>>,
    regex_partitions: Vec<Mutex<RegexMachinePartition>>,
    data_size: u64,
    regex_size: u64,
}

impl DataRepository {
    pub fn new(data_partition_count: u16, regex_partition_count: u16, regex_partition_capacity: u16) -> Self {
        let mut data_partitions: Vec<RwLock<DataPartition>> = Vec::with_capacity((
            if data_partition_count == 0 { DEFAULT_PARTITION_COUNT } else { data_partition_count }) as usize);
        for _ in 0..data_partition_count {
            data_partitions.push(RwLock::new(DataPartition::new()));
        }
        let mut regex_partitions: Vec<Mutex<RegexMachinePartition>> = Vec::with_capacity((
            if regex_partition_count == 0 { DEFAULT_PARTITION_COUNT } else { data_partition_count }) as usize);
        for _ in 0..regex_partition_count {
            regex_partitions.push(Mutex::new(RegexMachinePartition::new(regex_partition_capacity)));
        }
        let s1 = data_partitions.len() as u64;
        let s2 = regex_partitions.len() as u64;
        DataRepository{
            data_partitions,
            regex_partitions,
            data_size: s1,
            regex_size: s2
        }
    }
    pub async fn read(this: Arc<DataRepository>, key: &String) -> Option<Vec<u8>> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        let partition_lock = &this.data_partitions[(hash % this.data_size) as usize];
        let partition = partition_lock.read().await;
        if let Some((reference, limit)) = partition.map.get(key) {
            let lim = limit.fetch_sub(1, Ordering::AcqRel) - 1;
            if lim == 0 {
                let cloned_key = key.clone();
                let cloned_this = this.clone();
                tokio::spawn(async move { cloned_this.remove(&cloned_key).await; });
            }
            Some(reference.clone())
        } else {
            None
        }
    }
    pub async fn safe_read(&self, key: &String) -> Option<Vec<u8>> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        let partition_lock = &self.data_partitions[(hash % self.data_size) as usize];
        let partition = partition_lock.read().await;
        if let Some((reference, _)) = partition.map.get(key) {
            Some(reference.clone())
        } else {
            None
        }
    }
    pub async fn lifetime_read(&self, key: &String) -> Option<u32> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        let partition_lock = &self.data_partitions[(hash % self.data_size) as usize];
        let partition = partition_lock.read().await;
        if let Some((_, limit)) = partition.map.get(key) {
            Some(limit.load(Ordering::Acquire))
        } else {
            None
        }
    }
    pub async fn write(&self, key: String, value: Vec<u8>, read_limit: u32) {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        let partition_lock = &self.data_partitions[(hash % self.data_size) as usize];
        let mut partition = partition_lock.write().await;
        partition.map.insert(key, (value, AtomicU32::from(read_limit)));
    }
    pub async fn write_string(&self, key: String, value: String, read_limit: u32) {
        let bytes = Vec::from(value.as_bytes());
        self.write(key, bytes, read_limit).await;
    }
    pub async fn remove(&self, key: &String) {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        let partition_lock = &self.data_partitions[(hash % self.data_size) as usize];
        let mut partition = partition_lock.write().await;
        partition.map.remove(key);
    }
    fn get_regex(&self, pattern: &String) -> Result<Arc<Regex>, String> {
        let mut hasher = DefaultHasher::new();
        pattern.hash(&mut hasher);
        let hash = hasher.finish();
        let partition_lock = &self.regex_partitions[(hash % self.regex_size) as usize];
        let mut partition = partition_lock.lock().unwrap();
        let cache = &mut partition.map;
        if let Some(v) = cache.get(pattern) {
            return Ok(v.clone());
        }
        match Regex::new(pattern) {
            Ok(regex) => {
                let arc = Arc::new(regex);
                cache.put(pattern.clone(), arc.clone());
                Ok(arc)
            }
            Err(e) => {
                Err(e.to_string())
            }
        }
    }
    pub async fn match_remove(this: Arc<Self>, pattern: &String, limit: usize) -> Result<usize, String> {
        let regex_result = this.get_regex(pattern);
        if let Err(e) = regex_result {
            return Err(e);
        }
        let regex = regex_result.unwrap();
        let mut cleaned = 0usize;
        for partition_lock in &this.data_partitions {
            let partition = partition_lock.read().await;
            let immutable_map = &partition.map;
            for (key, _) in immutable_map.iter() {
                if !regex.is_match(key) {
                    continue;
                }
                let cloned_key = key.clone();
                let cloned_self = this.clone();
                tokio::spawn(async move {
                    cloned_self.remove(&cloned_key).await;
                });
                cleaned += 1;
                if cleaned >= limit {
                    return Ok(cleaned);
                }
            }
        }

        Ok(cleaned)
    }
    pub async fn clean(this: Arc<Self>) -> usize {
        let mut cleaned = 0usize;
        for partition_lock in &this.data_partitions {
            let mut partition = partition_lock.write().await;
            let map = &mut partition.map;
            cleaned += map.len();
            map.clear();
        }
        cleaned
    }
    pub async fn all_keys(&self) -> Vec<String> {
        let mut keys: Vec<String>  = Vec::new();
        for partition_lock in &self.data_partitions {
            let partition = partition_lock.read().await;
            for (key, _) in &partition.map {
                keys.push(key.clone());
            }
        }
        keys
    }
    pub async fn dump(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        for partition_lock in &self.data_partitions {
            let partition = partition_lock.read().await;
            for (key, (value, _)) in &partition.map {
                let mut key_bytes = key.clone().into_bytes();
                bytes.append(&mut Vec::from((key_bytes.len() as u64).to_be_bytes()));
                bytes.append(&mut key_bytes);
                let mut value_bytes = value.clone();
                bytes.append(&mut Vec::from((value_bytes.len() as u64).to_be_bytes()));
                bytes.append(&mut value_bytes);
            }
        }
        bytes
    }
    pub async fn dump_json(&self) -> HashMap<String, String> {
        let mut map: HashMap<String, String>  = HashMap::new();
        for partition_lock in &self.data_partitions {
            let partition = partition_lock.read().await;
            for (key, (value, _)) in &partition.map {
                let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(value);
                map.insert(key.clone(), encoded);
            }
        }
        map
    }
    pub async fn load(this: Arc<DataRepository>, dump: Vec<u8>, default_read_limit: u32) -> usize {
        let mut enrolled = 0usize;
        let mut cursor = 0usize;
        let seq_len = dump.len();
        while cursor + size_of::<u64>() <= seq_len {
            let key_len = u64::from_be_bytes(
                dump[cursor..cursor + size_of::<u64>()].try_into().unwrap()) as usize;
            cursor += size_of::<u64>();
            if cursor + key_len > seq_len { return enrolled; }
            match String::from_utf8(dump[cursor..cursor + key_len].to_vec()){
                Ok(key) => {
                    cursor += key_len;
                    if cursor + size_of::<u64>() > seq_len { return enrolled; }
                    let value_len = u64::from_be_bytes(
                        dump[cursor..cursor + size_of::<u64>()].try_into().unwrap()) as usize;
                    cursor += size_of::<u64>();
                    if cursor + value_len > seq_len { return enrolled; }
                    let value = dump[cursor..cursor + value_len].to_vec();
                    cursor += value_len;
                    enrolled += 1;
                    let cloned_self = this.clone();
                    tokio::spawn(async move {
                        cloned_self.write(key, value, default_read_limit).await;
                    });
                }
                Err(_) => {
                    cursor += key_len;
                    continue;
                }
            }
        }
        enrolled
    }
    pub async fn load_json(&self, dump: HashMap<String, String>, default_read_limit: u32) -> usize {
        let mut enrolled = 0usize;
        for (key, value) in dump {
            match base64::engine::general_purpose::STANDARD_NO_PAD.decode(value) {
                Ok(v) => {
                    self.write(key, v, default_read_limit).await;
                    enrolled += 1;
                },
                Err(_) =>{}
            }
        }
        enrolled
    }
}
