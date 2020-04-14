use crate::memlist::*;

#[derive(Debug)]
pub(crate) struct Storage {
    data_dir: String,
    pub mem_list: MemList,
    banks: Vec<Vec<u8>>,
}

impl Storage {
    pub fn new(data_dir: &str) -> Self {
        Self {
            data_dir: data_dir.into(),
            mem_list: MemList::new(data_dir),
            banks: Vec::new(),
        }
    }

    pub fn bank_data(&self, bank_id: usize) -> &[u8] {
        &self.banks[bank_id]
    }

    
}
