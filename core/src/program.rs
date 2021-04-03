use crate::{
    command::{Command, JmpType, OpType, ResetType},
    parts::GAME_PART_FIRST,
    slice_reader::SliceReader,
    staticres::*,
    video::Point,
    vm_context::VmContext,
};
use anyhow::{anyhow, bail, Result};
use std::{collections::HashMap, fmt, num::Wrapping};

const COLOR_BLACK: u8 = 0xFF;
const DEFAULT_ZOOM: u16 = 0x0040;

pub(crate) struct Program {
    id: usize,
    part_id: u16,
    code: Vec<u8>,
    active: bool,
    instructions: Vec<(usize, Command, usize)>,
    addr_ip: HashMap<u16, usize>,
    ip: usize,
    return_stack: Vec<usize>, // max 64
}

impl Program {
    pub fn new(id: usize, part_id: u16, mut code: Vec<u8>) -> Self {
        //printf("Jump : %X \n",_scriptPtr.pc-res->segBytecode);
        //FCS Whoever wrote this is patching the bytecode on the fly. This is ballzy !!
        if part_id == GAME_PART_FIRST {
            let ip = 0xCB9;
            // (0x0CB8) condJmp(0x80, VAR(41), VAR(30), 0xCD3)
            code[ip + 0x00] = 0x81;
            code[ip + 0x03] = 0x0D;
            code[ip + 0x04] = 0x24;
            // (0x0D4E) condJmp(0x4, VAR(50), 6, 0xDBC)
            code[ip + 0x99] = 0x0D;
            code[ip + 0x9A] = 0x5A;
            // printf("VirtualMachine::op_condJmp() bypassing protection");
            // printf("bytecode has been patched/n");

            //this->bypassProtection() ;
        }

        Self {
            id,
            part_id,
            code,
            instructions: Vec::new(),
            addr_ip: HashMap::new(),
            ip: 0,
            active: false,
            return_stack: Vec::new(),
        }
    }

    /// Get a reference to the program's ip.
    pub fn ip(&self) -> usize {
        self.ip
    }

    /// Get an address of the current command.
    pub fn addr(&self) -> usize {
        self.instructions[self.ip].0
    }

    pub fn goto_addr(&mut self, addr: u16) -> Result<()> {
        let oip = self.addr_ip.get(&addr);

        if let Some(ip) = oip {
            self.ip = *ip;
            Ok(())
        } else {
            let addresses = self
                .instructions
                .iter()
                .map(|(addr, _, _)| format!("{}", addr))
                .collect::<Vec<_>>()
                .join(", ");

            print!(
                "Cannot find command at address {}/0x{:04X}\nAvailable addresses: {}",
                addr, addr, addresses
            );
            Err(anyhow!(
                "Cannot find command at address {}/0x{:04X}",
                addr,
                addr
            ))
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn parse(&mut self) -> Result<()> {
        self.addr_ip.clear();
        self.instructions.clear();

        let mut slice_reader = SliceReader::new(&self.code);
        while slice_reader.can_read() {
            let addr = slice_reader.addr();
            let opcode = slice_reader.read_u8();
            let cmd = Command::parse(opcode, &mut slice_reader)?;

            // println!("{:05X}: {:?}", ip, cmd);

            self.addr_ip.insert(addr as u16, self.instructions.len());

            self.instructions
                .push((addr, cmd, slice_reader.addr() - addr));
        }

        Ok(())
    }

    pub fn start(&mut self) {
        self.ip = 0;
        self.active = true;
    }

    pub fn exec(&mut self, ctx: &mut VmContext) -> Result<()> {
        // if !self.active {
        //     return Ok(());
        // }

        while !ctx.goto_next_thread {
            let (addr, cmd, _) = &self.instructions[self.ip];
            let mut ip_incr = 1;

            // print!("{:04}/{:04X}: {:?}", self.ip, addr, cmd);
            print!("{:04X}: {:?}", addr, cmd);

            match cmd {
                Command::MovConst { var_id, val } => {
                    ctx.variables[var_id.0 as usize] = *val as i16;
                }
                Command::Mov { dst_id, src_id } => {
                    ctx.variables[dst_id.0 as usize] = ctx.variables[src_id.0 as usize];
                    print!(" -> {}", ctx.variables[dst_id.0 as usize]);
                }
                Command::Add { dst_id, src_id } => {
                    let v = Wrapping(ctx.variables[dst_id.0 as usize])
                        + Wrapping(ctx.variables[src_id.0 as usize]);

                    ctx.variables[dst_id.0 as usize] = v.0;
                    // ctx.variables[dst_id.0 as usize] += ctx.variables[src_id.0 as usize];

                    print!(" -> {}", ctx.variables[dst_id.0 as usize]);
                }
                Command::AddConst { var_id, val } => {
                    if self.part_id == 0x3E86 && self.ip == 0x6D48 {
                        // warning("VirtualMachine::op_addConst() hack for non-stop looping gun sound bug");
                        // the script 0x27 slot 0x17 doesn't stop the gun sound from looping, I
                        // don't really know why ; for now, let's play the 'stopping sound' like
                        // the other scripts do
                        //  (0x6D43) jmp(0x6CE5)
                        //  (0x6D46) break
                        //  (0x6D47) VAR(6) += -50
                        ctx.play_sound(0x5B, 1, 64, 1);
                    }

                    ctx.variables[var_id.0 as usize] += *val as i16;

                    print!(" -> {}", ctx.variables[var_id.0 as usize]);
                }
                Command::Call { offset } => {
                    self.return_stack.push(self.ip + 1); // TODO: use ip instead
                    self.goto_addr(*offset)?;
                    ip_incr = 0;
                }
                Command::Ret => {
                    self.ip = self
                        .return_stack
                        .pop()
                        .ok_or_else(|| anyhow!("Stack underflow"))?; // TODO: use ip instead
                    ip_incr = 0;
                }
                Command::PauseThread => ctx.goto_next_thread = true, // TODO: do we need to increase ip or can just return?
                Command::Jmp { offset } => {
                    self.goto_addr(*offset)?;
                    ip_incr = 0;
                }
                Command::SetVect { thr_id, offset } => {
                    ctx.threads_data[*thr_id as usize].requested_pc_offset = *offset
                }
                Command::Jnz { var_id, offset } => {
                    ctx.variables[var_id.0 as usize] -= 1;
                    if ctx.variables[var_id.0 as usize] != 0 {
                        self.goto_addr(*offset)?;
                        ip_incr = 0;
                        print!(" jmp");
                    }
                }
                Command::CondJmp {
                    jmp_type,
                    var_id,
                    op2,
                    offset,
                } => {
                    let val1 = ctx.variables[var_id.0 as usize];
                    let val2 = match op2 {
                        OpType::Var(var2_id) => ctx.variables[*var2_id as usize],
                        OpType::Val1(val) => *val as i16,
                        OpType::Val2(val) => *val as i16,
                    };
                    let cond = match jmp_type {
                        JmpType::Je => val1 == val2,
                        JmpType::Jne => val1 != val2,
                        JmpType::Jg => val1 > val2,
                        JmpType::Jge => val1 >= val2,
                        JmpType::Jl => val1 < val2,
                        JmpType::Jle => val1 <= val2,
                        JmpType::Unknown(_) => false, // NOTE: we should not come here
                    };

                    if cond {
                        print!(" -> {} ~ {} jmp", val1, val2);
                        self.goto_addr(*offset)?;
                        ip_incr = 0;
                    } else {
                        print!(" -> {} ~ {}", val1, val2);
                    }
                }
                Command::SetPalette { pal_id } => {
                    ctx.video.palette_id_requested = (*pal_id >> 8) as u8
                }
                Command::ResetThread {
                    reset_type,
                    first,
                    last,
                } => {
                    if *reset_type == ResetType::Delete {
                        for i in *first..=*last {
                            // TODO: fix magic numbers
                            ctx.threads_data[i as usize].requested_pc_offset = 0xFFFE;
                        }
                    } else {
                        let state_active = *reset_type == ResetType::Unfreeze;
                        for i in *first..=*last {
                            ctx.threads_data[i as usize].requested_state_active = state_active;
                        }
                    }
                }
                Command::SelectVideoPage { page_id } => {
                    ctx.video.change_page_off1(*page_id as usize)
                }
                Command::FillVideoPage { page_id, color } => {
                    ctx.video.fill_page(*page_id as usize, *color)
                }
                Command::CopyVideoPage {
                    src_page_id,
                    dst_page_id,
                } => ctx.video.copy_page(
                    *src_page_id as usize,
                    *dst_page_id as usize,
                    ctx.variables[VM_VARIABLE_SCROLL_Y],
                ),
                Command::BlitFramebuffer { page_id } => ctx.blit_framebuffer(*page_id as usize),
                Command::KillThread => {
                    self.active = false;
                    ctx.goto_next_thread = true;
                }
                Command::DrawString {
                    str_id,
                    x,
                    y,
                    color,
                } => ctx.video.draw_string(*color, *x as u16, *y as u16, *str_id),
                Command::Sub { dst_id, src_id } => {
                    ctx.variables[dst_id.0 as usize] -= ctx.variables[src_id.0 as usize]
                }
                Command::And { var_id, val } => {
                    ctx.variables[var_id.0 as usize] &= *val as i16;
                    print!(" -> {}", ctx.variables[var_id.0 as usize]);
                }
                Command::Or { var_id, val } => ctx.variables[var_id.0 as usize] |= *val as i16,
                Command::Shl { var_id, val } => ctx.variables[var_id.0 as usize] <<= *val,
                Command::Shr { var_id, val } => ctx.variables[var_id.0 as usize] >>= *val,
                Command::PlaySound {
                    res_id,
                    freq,
                    vol,
                    channel,
                } => ctx.play_sound(*res_id, *freq, *vol, *channel),
                Command::UpdateMemList { res_id } => ctx.update_mem_list(*res_id)?,
                Command::PlayMusic { res_id, delay, pos } => {
                    ctx.play_music(*res_id, *delay, *pos)?
                }
                Command::Video1 { offset, x, y } => {
                    ctx.video.set_data_page(true, *offset);
                    ctx.video.read_and_draw_polygon(
                        COLOR_BLACK,
                        DEFAULT_ZOOM,
                        Point::new(*x as i16, *y as i16),
                    );
                }
                Command::Video2 {
                    cinematic,
                    offset,
                    x,
                    y,
                    zoom,
                } => {
                    let x_val = match x {
                        OpType::Var(var_id) => ctx.variables[*var_id as usize],
                        OpType::Val1(val) => *val as i16,
                        OpType::Val2(val) => *val as i16,
                    };
                    let y_val = match y {
                        OpType::Var(var_id) => ctx.variables[*var_id as usize],
                        OpType::Val1(val) => *val as i16,
                        OpType::Val2(val) => *val as i16,
                    };
                    let zoom_val = match zoom {
                        OpType::Var(var_id) => ctx.variables[*var_id as usize] as u16,
                        OpType::Val1(val) => *val as u16,
                        OpType::Val2(val) => *val,
                    };

                    ctx.video.set_data_page(*cinematic, *offset);
                    ctx.video
                        .read_and_draw_polygon(0xFF, zoom_val, Point::new(x_val, y_val));
                }
            }

            println!("");

            self.ip += ip_incr;
        }
        Ok(())
    }
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (ip, cmd, size) in &self.instructions {
            let bytes: Vec<_> = self.code[*ip..*ip + *size]
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect();
            f.pad(&format!("{:05X}:\t  {:?}\n# {}\n", ip, cmd, bytes.join("")))?;
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

    // cargo test test_all_progs -- --nocapture

    #[test]
    fn test_all_progs() -> Result<()> {
        let proj_dir = proj_dir()?;
        let data_dir: String = data_dir()?.to_str().unwrap().into();
        let storage = Storage::new(&data_dir);
        let mut res = Resource::new(storage);

        res.init()?;

        let mut program_id = 0;
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
            let mut prog = Program::new(program_id, i as u16, data.into());

            prog.parse()?;

            let file_name = format!("resource-0x{:02x}.asm", i);
            let mut file = File::create(proj_dir.join(file_name))?;
            file.write_all(format!("{:?}", prog).as_bytes())?;

            program_id += 1;
        }

        Ok(())
    }
}
