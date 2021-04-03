use anyhow::Result;
use std::{num::Wrapping, path::PathBuf};

pub(crate) fn proj_dir() -> Result<PathBuf> {
    let mut dir = std::env::current_exe()?;

    // Go to project folder
    dir.pop();
    dir.pop();
    dir.pop();
    dir.pop();

    Ok(dir)
}

pub(crate) fn data_dir() -> Result<PathBuf> {
    let mut dir = proj_dir()?;

    dir.push("data");

    Ok(dir)
}

#[inline]
pub(crate) fn w_add_i16(v1: i16, v2: i16) -> i16 {
    (Wrapping(v1) + Wrapping(v2)).0
}

#[inline]
pub(crate) fn w_add_u32(v1: u32, v2: u32) -> u32 {
    (Wrapping(v1) + Wrapping(v2)).0
}

#[inline]
pub(crate) fn w_sub(v1: u8, v2: u8) -> u8 {
    (Wrapping(v1) - Wrapping(v2)).0
}

#[inline]
pub(crate) fn w_mul_i16(v1: i16, v2: i16) -> i16 {
    (Wrapping(v1) * Wrapping(v2)).0
}
