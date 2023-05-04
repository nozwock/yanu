use eyre::{bail, Result};
use itertools::Itertools;

pub mod nacp;
pub mod nca;
pub mod nsp;
pub mod ticket;
pub mod xci;

// Yes....this is vfs....

// No. of hexadecimal characters
pub const PROGRAMID_LEN: u8 = 16;

pub fn validate_program_id(program_id: &str) -> Result<()> {
    if program_id.len() == PROGRAMID_LEN as _ {
        Ok(())
    } else {
        bail!(
            "len: {} '{}' is invalid TitleID, it should be in hexadecimal \
            with a size of 8 bytes, i.e. 16 hexadecimal characters",
            program_id.len(),
            program_id
        )
    }
}

pub fn filter_out_lines(pat: &str, buf: &[u8]) -> String {
    let buf_str = String::from_utf8_lossy(buf);
    buf_str.lines().filter(|s| !s.contains(pat)).join("\n")
}

pub fn filter_out_key_mismatches(buf: &[u8]) -> String {
    filter_out_lines("Failed to match key", buf)
}
