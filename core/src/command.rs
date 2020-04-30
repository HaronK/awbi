use anyhow::{bail, Result};
use std::fmt;

pub(crate) enum Command {
    MovConst {
        var_id: u8,
        val: u16,
    },
    Mov {
        dst_id: u8,
        src_id: u8,
    },
    Add {
        dst_id: u8,
        src_id: u8,
    },
    AddConst {
        var_id: u8,
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
    SetSetVect {
        thr_id: u8,
        offset: u16,
    },
    Jnz {
        flag: u8,
        offset: u16,
    },
    CondJmp {
        opcode: u8,
        i: u8,
        c: u8,
        a: u8,
        offset: u16,
    },
    SetPallete {
        pal_id: u16,
    },
    ResetThread {
        thr_id: u8,
        i: u8,
        a: u8,
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
        dst_id: u8,
        src_id: u8,
    },
    And {
        var_id: u8,
        val: u16,
    },
    Or {
        var_id: u8,
        val: u16,
    },
    Shl {
        var_id: u8,
        val: u16,
    },
    Shr {
        var_id: u8,
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
        res_num: u16,
        delay: u16,
        pos: u8,
    },
}

impl Command {
    pub fn parse(opcode: u8, data: &[u8]) -> Result<Self> {
        let res = match opcode {
            0x00 => Self::MovConst {
                var_id: read_u8(data),
                val: read_u16(&data[1..]),
            },
            0x01 => Self::Mov {
                dst_id: read_u8(data),
                src_id: read_u8(&data[1..]),
            },
            0x02 => Self::Add {
                dst_id: read_u8(data),
                src_id: read_u8(&data[1..]),
            },
            0x03 => Self::AddConst {
                var_id: read_u8(data),
                val: read_u16(&data[1..]),
            },
            0x04 => Self::Call {
                offset: read_u16(data),
            },
            0x05 => Self::Ret,
            0x06 => Self::PauseThread,
            0x07 => Self::Jmp {
                offset: read_u16(data),
            },
            0x08 => Self::SetSetVect {
                thr_id: read_u8(data),
                offset: read_u16(&data[1..]),
            },
            0x09 => Self::Jnz {
                flag: read_u8(data),
                offset: read_u16(&data[1..]),
            },
            0x0A => {
                let oc = read_u8(data);
                let i = read_u8(&data[1..]);
                let c = read_u8(&data[2..]);
                let mut shift = 0;
                let a = if opcode & 0x80 != 0 {
                    0
                } else if opcode & 0x40 != 0 {
                    shift = 1;
                    read_u8(&data[3..])
                } else {
                    c
                };
                Self::CondJmp {
                    opcode: oc,
                    i,
                    c,
                    a,
                    offset: read_u16(&data[3 + shift..]),
                }
            }
            0x0B => Self::SetPallete {
                pal_id: read_u16(data),
            },
            0x0C => Self::ResetThread {
                thr_id: read_u8(data),
                i: read_u8(&data[1..]),
                a: read_u8(&data[2..]), // TODO: probably not always should be read. Compare with C++.
            },
            0x0D => Self::SelectVideoPage {
                page_id: read_u8(data),
            },
            0x0E => Self::FillVideoPage {
                page_id: read_u8(data),
                color: read_u8(&data[1..]),
            },
            0x0F => Self::CopyVideoPage {
                src_page_id: read_u8(data),
                dst_page_id: read_u8(&data[1..]),
            },
            0x10 => Self::BlitFramebuffer {
                page_id: read_u8(data),
            },
            0x11 => Self::KillThread,
            0x12 => Self::DrawString {
                str_id: read_u16(data),
                x: read_u8(&data[2..]),
                y: read_u8(&data[3..]),
                color: read_u8(&data[4..]),
            },
            0x13 => Self::Sub {
                dst_id: read_u8(data),
                src_id: read_u8(&data[1..]),
            },
            0x14 => Self::And {
                var_id: read_u8(data),
                val: read_u16(&data[1..]),
            },
            0x15 => Self::Or {
                var_id: read_u8(data),
                val: read_u16(&data[1..]),
            },
            0x16 => Self::Shl {
                var_id: read_u8(data),
                val: read_u16(&data[1..]),
            },
            0x17 => Self::Shr {
                var_id: read_u8(data),
                val: read_u16(&data[1..]),
            },
            0x18 => Self::PlaySound {
                res_id: read_u16(data),
                freq: read_u8(&data[2..]),
                vol: read_u8(&data[3..]),
                channel: read_u8(&data[4..]),
            },
            0x19 => Self::UpdateMemList {
                res_id: read_u16(data),
            },
            0x1A => Self::PlayMusic {
                res_num: read_u16(data),
                delay: read_u16(&data[2..]),
                pos: read_u8(&data[4..]),
            },
            _ => bail!("Command::parse() invalid opcode=0x{:02x}", opcode),
        };

        Ok(res)
    }
}

#[inline]
fn read_u8(data: &[u8]) -> u8 {
    data[0]
}

#[inline]
fn read_u16(data: &[u8]) -> u16 {
    u16::from_be_bytes([data[0], data[1]])
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::MovConst { var_id, val } => f.pad(&format!("MOVC {} {}", var_id, val)),
            Self::Mov { dst_id, src_id } => f.pad(&format!("MOV  {} {}", dst_id, src_id)),
            Self::Add { dst_id, src_id } => f.pad(&format!("ADD  {} {}", dst_id, src_id)),
            Self::AddConst { var_id, val } => f.pad(&format!("ADDC {} {}", var_id, val)),
            Self::Call { offset } => f.pad(&format!("CALL {:X}", offset)),
            Self::Ret => f.pad("RET"),
            Self::PauseThread => f.pad("PTHR"),
            Self::Jmp { offset } => f.pad(&format!("JMP  {:X}", offset)),
            Self::SetSetVect { thr_id, offset } => f.pad(&format!("SSV  {} {:X}", thr_id, offset)),
            Self::Jnz { flag, offset } => f.pad(&format!("JNZ  {} {:X}", flag, offset)),
            Self::CondJmp {
                opcode,
                i,
                c,
                a,
                offset,
            } => f.pad(&format!("CJMP {} {} {} {} {:X}", opcode, i, c, a, offset)),
            Self::SetPallete { pal_id } => f.pad(&format!("SPAL {}", pal_id)),
            Self::ResetThread { thr_id, i, a } => f.pad(&format!("RTHR {} {} {}", thr_id, i, a)),
            Self::SelectVideoPage { page_id } => f.pad(&format!("SVP  {}", page_id)),
            Self::FillVideoPage { page_id, color } => f.pad(&format!("FVP {} {}", page_id, color)),
            Self::CopyVideoPage {
                src_page_id,
                dst_page_id,
            } => f.pad(&format!("CVP  {} {}", src_page_id, dst_page_id)),
            Self::BlitFramebuffer { page_id } => f.pad(&format!("BFB  {}", page_id)),
            Self::KillThread => f.pad("KTHR"),
            Self::DrawString {
                str_id,
                x,
                y,
                color,
            } => f.pad(&format!("STR  {} {} {} {}", str_id, x, y, color)),
            Self::Sub { dst_id, src_id } => f.pad(&format!("SUB  {} {}", dst_id, src_id)),
            Self::And { var_id, val } => f.pad(&format!("AND  {} {}", var_id, val)),
            Self::Or { var_id, val } => f.pad(&format!("OR   {} {}", var_id, val)),
            Self::Shl { var_id, val } => f.pad(&format!("SHL  {} {}", var_id, val)),
            Self::Shr { var_id, val } => f.pad(&format!("SHR  {} {}", var_id, val)),
            Self::PlaySound {
                res_id,
                freq,
                vol,
                channel,
            } => f.pad(&format!("SND  {} {} {} {}", res_id, freq, vol, channel)),
            Self::UpdateMemList { res_id } => f.pad(&format!("UML  {}", res_id)),
            Self::PlayMusic {
                res_num,
                delay,
                pos,
            } => f.pad(&format!("MUS  {} {} {}", res_num, delay, pos)),
        }
    }
}
