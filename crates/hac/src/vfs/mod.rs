use eyre::{bail, Result};

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
