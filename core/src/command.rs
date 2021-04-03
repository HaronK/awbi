use crate::slice_reader::SliceReader;
use anyhow::{bail, Result};
use std::fmt;

pub(crate) struct OpVar(pub u8);

impl fmt::Debug for OpVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&format!("[{}]", var_name(self.0)))
    }
}

pub(crate) enum OpType {
    Var(u8),
    Val1(u8),
    Val2(u16),
}

impl fmt::Debug for OpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Var(addr) => f.pad(&format!("[{}]", var_name(addr))),
            Self::Val1(val) => f.pad(&format!("{}", val)),
            Self::Val2(val) => f.pad(&format!("{}", val)),
        }
    }
}

pub(crate) enum JmpType {
    Je,
    Jne,
    Jg,
    Jge,
    Jl,
    Jle,
    Unknown(u8),
}

impl JmpType {
    fn new(oc: u8) -> Result<Self> {
        let res = match oc {
            0 => Self::Je,
            1 => Self::Jne,
            2 => Self::Jg,
            3 => Self::Jge,
            4 => Self::Jl,
            5 => Self::Jle,
            _ => Self::Unknown(oc),
        };
        Ok(res)
    }
}

impl fmt::Debug for JmpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Je => f.pad("je"),
            Self::Jne => f.pad("jne"),
            Self::Jg => f.pad("jg"),
            Self::Jge => f.pad("jge"),
            Self::Jl => f.pad("jl"),
            Self::Jle => f.pad("jle"),
            Self::Unknown(code) => f.pad(&format!("unknown_jmp({})", code)),
        }
    }
}

#[derive(PartialEq)]
pub(crate) enum ResetType {
    None,
    Freeze,
    Unfreeze,
    Delete,
    Unknown(u8),
}

impl ResetType {
    fn new(oc: u8) -> Self {
        match oc {
            0 => Self::Freeze,
            1 => Self::Unfreeze,
            2 => Self::Delete,
            _ => Self::Unknown(oc),
        }
    }
}

impl fmt::Debug for ResetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::None => f.pad("NONE"),
            Self::Freeze => f.pad("freezeChannels"),
            Self::Unfreeze => f.pad("unfreezeChannels"),
            Self::Delete => f.pad("deleteChannels"),
            Self::Unknown(rt) => f.pad(&format!("unknown_reset_type({})", rt)),
        }
    }
}

fn var_name(id: u8) -> String {
    format!("0x{:02X}", id)
    // if let Some(name) = VARIABLE_NAME_BY_INDEX.get(&(id as usize)) {
    //     name.to_string()
    // } else {
    //     format!("0x{:02X}", id)
    // }
}

pub(crate) enum Command {
    MovConst {
        var_id: OpVar,
        val: u16,
    },
    Mov {
        dst_id: OpVar,
        src_id: OpVar,
    },
    Add {
        dst_id: OpVar,
        src_id: OpVar,
    },
    AddConst {
        var_id: OpVar,
        val: u16,
    },
    Call {
        offset: u16,
    },
    Ret,
    PauseThread,
    Jmp {
        offset: u16,
    },
    SetVect {
        thr_id: u8,
        offset: u16,
    },
    Jnz {
        var_id: OpVar,
        offset: u16,
    },
    CondJmp {
        jmp_type: JmpType,
        var_id: OpVar,
        op2: OpType,
        offset: u16,
    },
    SetPalette {
        pal_id: u16,
    },
    ResetThread {
        reset_type: ResetType,
        first: u8,
        last: u8,
    },
    SelectVideoPage {
        page_id: u8,
    },
    FillVideoPage {
        page_id: u8,
        color: u8,
    },
    CopyVideoPage {
        src_page_id: u8,
        dst_page_id: u8,
    },
    BlitFramebuffer {
        page_id: u8,
    },
    KillThread,
    DrawString {
        str_id: u16,
        x: u8,
        y: u8,
        color: u8,
    },
    Sub {
        dst_id: OpVar,
        src_id: OpVar,
    },
    And {
        var_id: OpVar,
        val: u16,
    },
    Or {
        var_id: OpVar,
        val: u16,
    },
    Shl {
        var_id: OpVar,
        val: u16,
    },
    Shr {
        var_id: OpVar,
        val: u16,
    },
    PlaySound {
        res_id: u16,
        freq: u8,
        vol: u8,
        channel: u8,
    },
    UpdateMemList {
        res_id: u16,
    },
    PlayMusic {
        res_id: u16,
        delay: u16,
        pos: u8,
    },
    Video1 {
        offset: usize,
        x: u8,
        y: u8,
    },
    Video2 {
        cinematic: bool,
        offset: usize,
        x: OpType,
        y: OpType,
        zoom: OpType,
    },
}

impl Command {
    pub fn parse(opcode: u8, sr: &mut SliceReader) -> Result<Self> {
        let res = match opcode {
            0x00 => Self::MovConst {
                var_id: OpVar(sr.read_u8()),
                val: sr.read_u16(),
            },
            0x01 => Self::Mov {
                dst_id: OpVar(sr.read_u8()),
                src_id: OpVar(sr.read_u8()),
            },
            0x02 => Self::Add {
                dst_id: OpVar(sr.read_u8()),
                src_id: OpVar(sr.read_u8()),
            },
            0x03 => Self::AddConst {
                var_id: OpVar(sr.read_u8()),
                val: sr.read_u16(),
            },
            0x04 => Self::Call {
                offset: sr.read_u16(),
            },
            0x05 => Self::Ret,
            0x06 => Self::PauseThread,
            0x07 => Self::Jmp {
                offset: sr.read_u16(),
            },
            0x08 => Self::SetVect {
                thr_id: sr.read_u8(),
                offset: sr.read_u16(),
            },
            0x09 => Self::Jnz {
                var_id: OpVar(sr.read_u8()),
                offset: sr.read_u16(),
            },
            0x0A => {
                let oc = sr.read_u8();
                let var_id = OpVar(sr.read_u8());
                let c = sr.read_u8();
                let op2 = if oc & 0x80 != 0 {
                    OpType::Var(c)
                } else if oc & 0x40 != 0 {
                    OpType::Val2((c as u16) * 256 + sr.read_u8() as u16)
                } else {
                    OpType::Val1(c)
                };

                let jmp_type = JmpType::new(oc & 7)?;

                if let JmpType::Unknown(jt) = jmp_type {
                    bail!("Command::parse() invalid jmp opcode {}", jt)
                }

                Self::CondJmp {
                    jmp_type,
                    var_id,
                    op2,
                    offset: sr.read_u16(),
                }
            }
            0x0B => Self::SetPalette {
                pal_id: sr.read_u16(),
            },
            0x0C => {
                let first = sr.read_u8();
                let last = sr.read_u8();

                if last < first {
                    println!("Command::parse(): first({}) > last({})", first, last);

                    Self::ResetThread {
                        reset_type: ResetType::None,
                        first,
                        last,
                    }
                } else {
                    let reset_type = ResetType::new(sr.read_u8());

                    if let ResetType::Unknown(rt) = reset_type {
                        println!("Command::parse() invalid resetThread opcode {}", rt);
                    }

                    Self::ResetThread {
                        reset_type,
                        first,
                        last,
                    }
                }
            }
            0x0D => Self::SelectVideoPage {
                page_id: sr.read_u8(),
            },
            0x0E => Self::FillVideoPage {
                page_id: sr.read_u8(),
                color: sr.read_u8(),
            },
            0x0F => Self::CopyVideoPage {
                src_page_id: sr.read_u8(),
                dst_page_id: sr.read_u8(),
            },
            0x10 => Self::BlitFramebuffer {
                page_id: sr.read_u8(),
            },
            0x11 => Self::KillThread,
            0x12 => Self::DrawString {
                str_id: sr.read_u16(),
                x: sr.read_u8(),
                y: sr.read_u8(),
                color: sr.read_u8(),
            },
            0x13 => Self::Sub {
                dst_id: OpVar(sr.read_u8()),
                src_id: OpVar(sr.read_u8()),
            },
            0x14 => Self::And {
                var_id: OpVar(sr.read_u8()),
                val: sr.read_u16(),
            },
            0x15 => Self::Or {
                var_id: OpVar(sr.read_u8()),
                val: sr.read_u16(),
            },
            0x16 => Self::Shl {
                var_id: OpVar(sr.read_u8()),
                val: sr.read_u16(),
            },
            0x17 => Self::Shr {
                var_id: OpVar(sr.read_u8()),
                val: sr.read_u16(),
            },
            0x18 => Self::PlaySound {
                res_id: sr.read_u16(),
                freq: sr.read_u8(),
                vol: sr.read_u8(),
                channel: sr.read_u8(),
            },
            0x19 => Self::UpdateMemList {
                res_id: sr.read_u16(),
            },
            0x1A => Self::PlayMusic {
                res_id: sr.read_u16(),
                delay: sr.read_u16(),
                pos: sr.read_u8(),
            },
            _ => {
                if opcode & 0x80 != 0 {
                    let offset =
                        ((((opcode as usize) << 8) | (sr.read_u8() as usize)) * 2) & 0xFFFF;
                    let mut x = sr.read_u8();
                    let mut y = sr.read_u8();

                    if y > 199 {
                        y = 199;
                        x += y - 199;
                    }

                    Self::Video1 { offset, x, y }
                } else if opcode & 0x40 != 0 {
                    let offset = (sr.read_u16() as usize) * 2;
                    let x_val = sr.read_u8();

                    let x = if opcode & 0x20 == 0 {
                        if opcode & 0x10 == 0 {
                            OpType::Val2(((x_val as u16) << 8) | (sr.read_u8() as u16))
                        } else {
                            OpType::Var(x_val)
                        }
                    } else if opcode & 0x10 != 0 {
                        OpType::Val2((x_val as u16) + 0x100)
                    } else {
                        OpType::Val1(x_val)
                    };

                    let y_val = sr.read_u8();
                    let y = if opcode & 8 == 0 {
                        if opcode & 4 == 0 {
                            OpType::Val2(((y_val as u16) << 8) | (sr.read_u8() as u16))
                        } else {
                            OpType::Var(y_val)
                        }
                    } else {
                        OpType::Val1(y_val)
                    };

                    let mut cinematic = true;
                    let zoom = if opcode & 2 == 0 {
                        if opcode & 1 == 0 {
                            OpType::Val1(0x40)
                        } else {
                            OpType::Var(sr.read_u8())
                        }
                    } else if opcode & 1 != 0 {
                        cinematic = false;
                        OpType::Val1(0x40)
                    } else {
                        OpType::Val1(sr.read_u8())
                    };

                    Self::Video2 {
                        cinematic,
                        offset,
                        x,
                        y,
                        zoom,
                    }
                } else {
                    bail!("Command::parse() invalid opcode=0x{:02X}", opcode);
                }
            }
        };

        Ok(res)
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MovConst { var_id, val } => f.pad(&format!("mov {:?}, {}", var_id, val)),
            Self::Mov { dst_id, src_id } => f.pad(&format!("mov {:?}, {:?}", dst_id, src_id)),
            Self::Add { dst_id, src_id } => f.pad(&format!("add {:?}, {:?}", dst_id, src_id)),
            Self::AddConst { var_id, val } => f.pad(&format!("add {:?}, {}", var_id, val)),
            Self::Call { offset } => f.pad(&format!("call 0x{:04X}", offset)),
            Self::Ret => f.pad("ret"),
            Self::PauseThread => f.pad("pauseThread"),
            Self::Jmp { offset } => f.pad(&format!("jmp 0x{:04X}", offset)),
            Self::SetVect { thr_id, offset } => f.pad(&format!(
                "setvec channel:{}, address:0x{:04X}",
                thr_id, offset
            )),
            Self::Jnz { var_id, offset } => f.pad(&format!("jnz {:?}, 0x{:04X}", var_id, offset)),
            Self::CondJmp {
                jmp_type,
                var_id,
                op2,
                offset,
            } => f.pad(&format!(
                "{:?} {:?}, {:?}, 0x{:04X}",
                jmp_type, var_id, op2, offset
            )),
            Self::SetPalette { pal_id } => f.pad(&format!("setPalette {}", pal_id)),
            Self::ResetThread {
                reset_type,
                first,
                last,
            } => f.pad(&format!("{:?}, first:{}, last:{}", reset_type, first, last)),
            Self::SelectVideoPage { page_id } => f.pad(&format!("selectVideoPage {}", page_id)),
            Self::FillVideoPage { page_id, color } => {
                f.pad(&format!("fillVideoPage {}, color:{}", page_id, color))
            }
            Self::CopyVideoPage {
                src_page_id,
                dst_page_id,
            } => f.pad(&format!(
                "copyVideoPage src:{}, dst:{}",
                src_page_id, dst_page_id
            )),
            Self::BlitFramebuffer { page_id } => f.pad(&format!("blitFramebuffer {}", page_id)),
            Self::KillThread => f.pad("killThread"),
            Self::DrawString {
                str_id,
                x,
                y,
                color,
            } => f.pad(&format!(
                "drawString id:{}, x:{}, y:{}, color:{}",
                str_id, x, y, color
            )),
            // f.pad(&format!(
            //     "drawString id:{}, x:{}, y:{}, color:{}  \"{}\"",
            //     str_id,
            //     x,
            //     y,
            //     color,
            //     STRINGS_TABLE_ENG.get(&(*str_id as u16)).unwrap_or(&"")
            // )),
            Self::Sub { dst_id, src_id } => f.pad(&format!("sub {:?}, {:?}", dst_id, src_id)),
            Self::And { var_id, val } => f.pad(&format!("and {:?}, {}", var_id, val)),
            Self::Or { var_id, val } => f.pad(&format!("or {:?}, {}", var_id, val)),
            Self::Shl { var_id, val } => f.pad(&format!("shl {:?}, {}", var_id, val)),
            Self::Shr { var_id, val } => f.pad(&format!("shr {:?}, {}", var_id, val)),
            Self::PlaySound {
                res_id,
                freq,
                vol,
                channel,
            } => f.pad(&format!(
                "play id:{}, freq:{}, vol:{}, channel:{}",
                res_id, freq, vol, channel
            )),
            Self::UpdateMemList { res_id } => f.pad(&format!("load id:{}", res_id)),
            Self::PlayMusic { res_id, delay, pos } => {
                f.pad(&format!("song id:{}, delay:{}, pos:{}", res_id, delay, pos))
            }
            Self::Video1 { offset, x, y } => {
                f.pad(&format!("video1: off={} x={} y={}", offset, x, y))
            }
            Self::Video2 {
                cinematic: _,
                offset,
                x,
                y,
                zoom,
            } => f.pad(&format!(
                "video2: off={} x={:?} y={:?} zoom:{:?}",
                offset, x, y, zoom
            )),
        }
    }
}
