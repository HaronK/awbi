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
    SetVect {
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
    Video1 {
        offset: usize,
        x: u8,
        y: u8,
    },
    Video2 {
        opcode: u8,
        offset: usize,
        x: u8,
        x_corr: u8,
        y: u8,
        y_corr: u8,
        zoom: u8,
        size: usize,
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
            0x08 => Self::SetVect {
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
            _ => {
                if opcode & 0x80 != 0 {
                    Self::Video1 {
                        offset: (((opcode as usize) << 8) | (read_u8(data) as usize)) * 2,
                        x: read_u8(&data[1..]),
                        y: read_u8(&data[2..]),
                    }
                } else if opcode & 0x40 != 0 {
                    let mut shift = 0;
                    let offset = (read_u16(data) as usize) * 2;
                    let x = read_u8(&data[2..]);
                    let x_corr = if opcode & 0x20 == 0 && opcode & 0x10 == 0 {
                        shift = 1;
                        read_u8(&data[3..])
                    } else {
                        0
                    };
                    let y = read_u8(&data[3 + shift..]);
                    let y_corr = if opcode & 8 == 0 && opcode & 4 == 0 {
                        shift += 1;
                        read_u8(&data[3 + shift..])
                    } else {
                        0
                    };
                    let zoom = read_u8(&data[4 + shift..]);

                    Self::Video2 {
                        opcode,
                        offset,
                        x,
                        x_corr,
                        y,
                        y_corr,
                        zoom,
                        size: 5 + shift,
                    }
                } else {
                    bail!("Command::parse() invalid opcode=0x{:02X}", opcode);
                }
            }
        };

        Ok(res)
    }

    pub fn args_size(&self) -> usize {
        match *self {
            Self::MovConst { var_id: _, val: _ } => 3,
            Self::Mov {
                dst_id: _,
                src_id: _,
            } => 2,
            Self::Add {
                dst_id: _,
                src_id: _,
            } => 2,
            Self::AddConst { var_id: _, val: _ } => 3,
            Self::Call { offset: _ } => 2,
            Self::Ret => 0,
            Self::PauseThread => 0,
            Self::Jmp { offset: _ } => 2,
            Self::SetVect {
                thr_id: _,
                offset: _,
            } => 3,
            Self::Jnz { flag: _, offset: _ } => 3,
            Self::CondJmp {
                opcode,
                i: _,
                c: _,
                a: _,
                offset: _,
            } => {
                if opcode & (0x80 + 0x40) == 0x40 {
                    6
                } else {
                    5
                }
            }
            Self::SetPallete { pal_id: _ } => 2,
            Self::ResetThread {
                thr_id: _,
                i: _,
                a: _,
            } => 3, // TODO: check this with C++
            Self::SelectVideoPage { page_id: _ } => 1,
            Self::FillVideoPage {
                page_id: _,
                color: _,
            } => 2,
            Self::CopyVideoPage {
                src_page_id: _,
                dst_page_id: _,
            } => 2,
            Self::BlitFramebuffer { page_id: _ } => 1,
            Self::KillThread => 0,
            Self::DrawString {
                str_id: _,
                x: _,
                y: _,
                color: _,
            } => 5,
            Self::Sub {
                dst_id: _,
                src_id: _,
            } => 2,
            Self::And { var_id: _, val: _ } => 3,
            Self::Or { var_id: _, val: _ } => 3,
            Self::Shl { var_id: _, val: _ } => 3,
            Self::Shr { var_id: _, val: _ } => 3,
            Self::PlaySound {
                res_id: _,
                freq: _,
                vol: _,
                channel: _,
            } => 5,
            Self::UpdateMemList { res_id: _ } => 2,
            Self::PlayMusic {
                res_num: _,
                delay: _,
                pos: _,
            } => 5,
            Self::Video1 {
                offset: _,
                x: _,
                y: _,
            } => 3,
            Self::Video2 {
                opcode: _,
                offset: _,
                x: _,
                x_corr: _,
                y: _,
                y_corr: _,
                zoom: _,
                size,
            } => size,
        }
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
            Self::MovConst { var_id, val } => {
                f.pad(&format!("mov  [0x{:02X}], 0x{:04X}", var_id, val))
            }
            Self::Mov { dst_id, src_id } => {
                f.pad(&format!("mov  [0x{:02X}], [0x{:02X}]", dst_id, src_id))
            }
            Self::Add { dst_id, src_id } => {
                f.pad(&format!("add  [0x{:02X}], [0x{:02X}]", dst_id, src_id))
            }
            Self::AddConst { var_id, val } => {
                f.pad(&format!("add  [0x{:02X}], 0x{:04X}", var_id, val))
            }
            Self::Call { offset } => f.pad(&format!("call 0x{:04X}", offset)),
            Self::Ret => f.pad("ret"),
            Self::PauseThread => f.pad("break"),
            Self::Jmp { offset } => f.pad(&format!("jmp  0x{:04X}", offset)),
            Self::SetVect { thr_id, offset } => f.pad(&format!(
                "vec  channel:0x{:02X}, address:0x{:04X}",
                thr_id, offset
            )),
            Self::Jnz { flag, offset } => {
                f.pad(&format!("jnz  [0x{:02X}], 0x{:04X}", flag, offset))
            }
            Self::CondJmp {
                opcode,
                i,
                c,
                a,
                offset,
            } => f.pad(&format!(
                "cjmp {} {} [0x{:02X}], 0x{:02X}, 0x{:04X}",
                opcode, c, i, a, offset
            )),
            Self::SetPallete { pal_id } => f.pad(&format!("setPallete 0x{:04X}", pal_id)),
            Self::ResetThread { thr_id, i, a } => f.pad(&format!(
                "deleteChannels first:ox{:02X}, last:0x{:02X}, {}",
                thr_id, i, a
            )),
            Self::SelectVideoPage { page_id } => {
                f.pad(&format!("selectVideoPage 0x{:02X}", page_id))
            }
            Self::FillVideoPage { page_id, color } => f.pad(&format!(
                "fillVideoPage 0x{:02X}, color:0x{:02X}",
                page_id, color
            )),
            Self::CopyVideoPage {
                src_page_id,
                dst_page_id,
            } => f.pad(&format!(
                "copyVideoPage src:0x{:02X}, dst:0x{:02X}",
                src_page_id, dst_page_id
            )),
            Self::BlitFramebuffer { page_id } => {
                f.pad(&format!("blitFramebuffer 0x{:02X}", page_id))
            }
            Self::KillThread => f.pad("killChannel"),
            Self::DrawString {
                str_id,
                x,
                y,
                color,
            } => f.pad(&format!(
                "text id:0x{:04X}, x:{}, y:{}, color:0x{:02X}",
                str_id, x, y, color
            )),
            Self::Sub { dst_id, src_id } => {
                f.pad(&format!("sub  [0x{:02X}] [0x{:02X}]", dst_id, src_id))
            }
            Self::And { var_id, val } => f.pad(&format!("and  [0x{:02X}] 0x{:04X}", var_id, val)),
            Self::Or { var_id, val } => f.pad(&format!("or   [0x{:02X}] 0x{:04X}", var_id, val)),
            Self::Shl { var_id, val } => f.pad(&format!("shl  [0x{:02X}] 0x{:04X}", var_id, val)),
            Self::Shr { var_id, val } => f.pad(&format!("shr  [0x{:02X}] 0x{:04X}", var_id, val)),
            Self::PlaySound {
                res_id,
                freq,
                vol,
                channel,
            } => f.pad(&format!(
                "play id:0x{:04X}, freq:0x{:02X}, vol:0x{:02X}, channel:0x{:02X}",
                res_id, freq, vol, channel
            )),
            Self::UpdateMemList { res_id } => f.pad(&format!("load id:0x{:04X}", res_id)),
            Self::PlayMusic {
                res_num,
                delay,
                pos,
            } => f.pad(&format!(
                "song id:0x{:04X}, delay:0x{:04X}, pos:0x{:02X}",
                res_num, delay, pos
            )),
            Self::Video1 { offset, x, y } => {
                f.pad(&format!("video1: off=0x{:04X} x={} y={}", offset, x, y))
            }
            Self::Video2 {
                opcode,
                offset,
                x,
                x_corr,
                y,
                y_corr,
                zoom,
                size: _,
            } => f.pad(&format!(
                "video2: {} off=0x{:04X} x=[0x{:02X}] {} y=[0x{:02X}] {} zoom:0x{:02X}",
                opcode, offset, x, x_corr, y, y_corr, zoom
            )),
        }
    }
}
