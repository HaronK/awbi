use crate::command::Command;
use std::fmt;

pub(crate) struct Program {
    code: Vec<u8>,
}

impl Program {
    pub fn new(code: Vec<u8>) -> Self {
        Self { code }
    }
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ip = 0;

        while ip < self.code.len() {
            let opcode = self.code[ip];
            let cmd = Command::parse(opcode, &self.code[ip + 1..]).map_err(|e| {
                println!("ERROR: [{}:{}] {:?}", ip, self.code.len(), e);
                fmt::Error
            })?;
            // let cmd = match Command::parse(opcode, &self.code[ip + 1..]) {
            //     Ok(cmd) => cmd,
            //     Err(err) => {
            //         println!("ERROR: [{:05X}:{:05X}] {:?}", ip, self.code.len(), err);
            //         break;
            //     }
            // };

            f.pad(&format!("{:05X}:\t  {:?}\n", ip, cmd))?;

            ip += 1 + cmd.args_size();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memlist::ResType, resource::Resource, storage::Storage, util::*};
    use anyhow::Result;
    use std::fs::File;
    use std::io::prelude::*;

    #[test]
    fn test_all_progs() -> Result<()> {
        let proj_dir = proj_dir()?;
        let data_dir: String = data_dir()?.to_str().unwrap().into();
        let storage = Storage::new(&data_dir);
        let mut res = Resource::new(storage);

        res.init()?;

        for (i, me) in res
            .storage
            .mem_list
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| (**e).res_type == ResType::Bytecode)
        {
            println!(
                "Program({:02x}): bank_id: {}, bank_offset: {}, packed_size: {}, size: {}",
                i, me.bank_id, me.bank_offset, me.packed_size, me.size
            );

            let data = me.read_bank();
            let prog = Program::new(data.into());

            // println!("{:?}", prog);

            let file_name = format!("resource-0x{:02x}.asm", i);
            let mut file = File::create(proj_dir.join(file_name))?;
            file.write_all(format!("{:?}", prog).as_bytes())?;
        }

        Ok(())
    }
}
