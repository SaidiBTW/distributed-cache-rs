use std::collections::HashMap;

use crate::{
    models::arena::{Arena, ArenaPtr},
    raft::Node,
};

pub type Cache = CacheStore;

pub struct CacheStore {
    cache: HashMap<Vec<u8>, ArenaPtr>,
    arena: Arena,
}

impl CacheStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::new(),
            arena: Arena::new(capacity),
        }
    }

    pub fn set(&mut self, key: Vec<u8>, value: &[u8]) -> Result<(), &'static str> {
        if self.arena.usage_ratio() >= 0.85 {
            self.compact();

            if self.arena.usage_ratio() > 0.85 {
                return Err("OOM: Arena full even after compaction");
            }
        }

        if let Some(ptr) = self.arena.allocate(value) {
            self.cache.insert(key, ptr);
            Ok(())
        } else {
            Err("Failed to allocate in arena")
        }
    }
    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        match self.cache.get(key) {
            Some(ptr) => {
                let value = self.arena.read(*ptr);
                return Some(value);
            }
            None => None,
        }
    }
    pub fn delete(&mut self, key: &[u8]) -> bool {
        //You can safely ignore the Arena during this step
        //since the compaction process uses only the existing
        //data in the cache to construct itself
        match self.cache.remove(key) {
            Some(_) => true,
            None => false,
        }
    }

    //Garbage Collection
    fn compact(&mut self) {
        let mut new_arena = Arena::new(self.arena.capacity);
        for (_key, ptr) in self.cache.iter_mut() {
            let data = self.arena.read(*ptr);
            let new_ptr = new_arena
                .allocate(data)
                .expect("New arena cannot hold existing live data");

            *ptr = new_ptr;
        }

        self.arena = new_arena;
        println!(
            "Compaction complete. New usage: {:.2}",
            self.arena.usage_ratio()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_reclaims_dead_space() {
        let mut store = CacheStore::new(100);

        store
            .set(b"baseline_key".to_vec(), b"baseline_data")
            .unwrap();

        for i in 0..10 {
            let val = format!("data_v{:02}", i);
            let res = store.set(b"churn_key".to_vec(), val.as_bytes());
            assert!(res.is_ok(), "Failed at iteration {}", i);
            println!("{}", store.arena.usage_ratio());
        }

        assert_eq!(store.get(b"baseline_key").unwrap(), b"baseline_data");

        assert_eq!(store.get(b"churn_key").unwrap(), b"data_v09");

        assert!(store.arena.usage_ratio() < 0.8, "Memory was not reclaimed");
    }
}
