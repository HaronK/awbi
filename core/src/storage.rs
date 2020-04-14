use crate::{bank::Bank, memlist::*};
use anyhow::Result;
use std::io::Cursor;

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

    pub fn load(&mut self) -> Result<()> {
        let mut bank = Bank::default();
        for i in 1..=13 {
            let mut f = bank.read_bank(&self.data_dir, i)?;
            let data = f.read_all()?;
            self.banks.push(data);
        }

        self.mem_list.load()?;

        for me in &mut self.mem_list.entries {
            let mut cursor = Cursor::new(&self.banks[me.bank_id as usize - 1]);
            let data = bank.read_entry_data(&mut cursor, &me)?;
            me.buffer = data;
        }

        Ok(())
    }

    pub fn get_max_rank_entry_to_load(&mut self) -> Option<&mut MemEntry> {
        let mut mem_entry: Option<&mut MemEntry> = None;
        let mut max_num = 0;

        for me in &mut self.mem_list.entries {
            if me.state == MemEntryState::LoadMe && max_num <= me.rank_num {
                max_num = me.rank_num;
                mem_entry = Some(me);
            }
        }

        mem_entry
    }

    pub fn get_loaded_entry_with_offset(&self, offset: usize) -> Option<(usize, &MemEntry)> {
        let mut mem_entry = None;

        for (i, me) in self.mem_list.entries.iter().enumerate() {
            if me.state == MemEntryState::Loaded && me.buf_offset == offset {
                mem_entry = Some((i, me));
                break; // TODO: check this
            }
        }

        mem_entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::data_dir;

    #[test]
    fn test_storage_load() -> Result<()> {
        let data_dir = data_dir()?;
        let mut storage = Storage::new(&data_dir.to_str().unwrap());

        storage.load()?;

        // println!("Storage:\n{:?}", storage);

        Ok(())
    }
}
