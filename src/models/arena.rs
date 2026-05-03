//Pointer to a location in your Arean
#[derive(Clone, Copy, Debug)]
pub struct ArenaPtr {
    pub offset: usize,
    pub length: usize,
}

pub struct Arena {
    pub memory: Vec<u8>,
    pub capacity: usize,
    pub alloc_offset: usize, // Points to the next free byte
}

impl Arena {
    pub fn new(capacity: usize) -> Self {
        Self {
            memory: vec![0; capacity],
            capacity,
            alloc_offset: 0,
        }
    }

    pub fn allocate(&mut self, data: &[u8]) -> Option<ArenaPtr> {
        if self.alloc_offset + data.len() > self.memory.len() {
            return None;
        }

        let start = self.alloc_offset;
        let end = start + data.len();

        self.memory[start..end].copy_from_slice(data);
        self.alloc_offset = end;

        Some(ArenaPtr {
            offset: start,
            length: data.len(),
        })
    }

    pub fn read(&self, ptr: ArenaPtr) -> &[u8] {
        &self.memory[ptr.offset..(ptr.offset + ptr.length)]
    }

    pub fn usage_ratio(&self) -> f64 {
        self.alloc_offset as f64 / self.capacity as f64
    }
}
