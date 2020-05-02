use crate::{
    command::{Command, JmpType, OpType, ResetType},
    parts::GAME_PART_FIRST,
    staticres::*,
    video::Point,
    vm_context::VmContext,
};
use anyhow::{anyhow, bail, Result};
use std::{collections::HashMap, fmt};

const COLOR_BLACK: u8 = 0xFF;
const DEFAULT_ZOOM: u16 = 0x0040;

pub(crate) struct Program {
    part_id: u16,
    code: Vec<u8>,
    active: bool,
    instructions: Vec<(usize, Command)>,
    addr_ip: HashMap<u16, usize>,
    ip: usize,
    return_stack: Vec<usize>, // max 64
}

impl Program {
    pub fn new(part_id: u16, mut code: Vec<u8>) -> Self {
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
            part_id,
            code,
            instructions: Vec::new(),
            addr_ip: HashMap::new(),
            ip: 0,
            active: false,
            return_stack: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Result<()> {
        self.instructions.clear();

        let mut ip = 0;
        while ip < self.code.len() {
            let opcode = self.code[ip];
            let cmd = Command::parse(opcode, &self.code[ip + 1..])?;
            let args_size = cmd.args_size();

            self.addr_ip.insert(ip as u16, self.instructions.len());

            self.instructions.push((ip, cmd));

            ip += 1 + args_size;
        }

        // TODO: replace jump instructions offsets with ip offsets using addr_ip

        Ok(())
    }

    pub fn exec(&mut self, ctx: &mut VmContext) -> Result<()> {
        if !self.active {
            return Ok(());
        }

        let mut run = true;
        while run {
            let (addr, cmd) = &self.instructions[self.ip];
            match cmd {
                Command::MovConst { var_id, val } => {
                    ctx.variables[var_id.0 as usize] = *val as i16;
                }
                Command::Mov { dst_id, src_id } => {
                    ctx.variables[dst_id.0 as usize] = ctx.variables[src_id.0 as usize];
                }
                Command::Add { dst_id, src_id } => {
                    ctx.variables[dst_id.0 as usize] += ctx.variables[src_id.0 as usize];
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
                }
                Command::Call { offset } => {
                    self.return_stack.push(*addr as usize); // TODO: use ip instead
                    self.ip = self.addr_ip[&(*addr as u16 + *offset)];
                    continue;
                }
                Command::Ret => {
                    let ret_addr = self
                        .return_stack
                        .pop()
                        .ok_or_else(|| anyhow!("Stack underflow"))?; // TODO: use ip instead
                    self.ip = self.addr_ip[&(ret_addr as u16)];
                    continue;
                }
                Command::PauseThread => run = false, // TODO: do we need to increase ip or can just return?
                Command::Jmp { offset } => {
                    self.ip = self.addr_ip[&(*addr as u16 + *offset)];
                    continue;
                }
                Command::SetVect { thr_id, offset } => {
                    ctx.threads_data[*thr_id as usize].requested_pc_offset = *offset
                }
                Command::Jnz { var_id, offset } => {
                    ctx.variables[var_id.0 as usize] -= 1;
                    if ctx.variables[var_id.0 as usize] != 0 {
                        self.ip = self.addr_ip[&(*addr as u16 + *offset)];
                        continue;
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
                        self.ip = self.addr_ip[&(*addr as u16 + *offset)];
                        continue;
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
                            // TODO: fix magic numbers
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
                    run = false; // TODO: do we need to increase ip or can just return?
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
                Command::And { var_id, val } => ctx.variables[var_id.0 as usize] &= *val as i16,
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
                    size: _,
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

            self.ip += 1;
        }
        Ok(())
    }
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (ip, cmd) in &self.instructions {
            f.pad(&format!("{:05X}:\t  {:?}\n", ip, cmd))?;
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
            let mut prog = Program::new(i as u16, data.into());

            prog.parse()?;

            let file_name = format!("resource-0x{:02x}.asm", i);
            let mut file = File::create(proj_dir.join(file_name))?;
            file.write_all(format!("{:?}", prog).as_bytes())?;
        }

        Ok(())
    }
}
