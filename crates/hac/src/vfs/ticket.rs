//! https://switchbrew.org/wiki/Ticket
//!
//! Contains method for extracting TitleKey from Tickets, a format used to store an encrypted title key.
//!
//! Cheap implementation only supporting 'common' Title key type.

use eyre::Result;
use fs_err as fs;
use std::{
    fmt,
    io::{self, Read, Seek},
    path::Path,
};
use tracing::{debug, info};

#[derive(Debug, Default, Clone)]
pub struct TitleKey {
    rights_id: [u8; 0x10],
    title_key: [u8; 0x10], // for Common TitleKey type
}

impl TitleKey {
    const RIGHTS_ID_OFFSET: usize = 0x2a0;
    const TITLE_KEY_OFFSET: usize = 0x180;
}

impl fmt::Display for TitleKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}={}",
            hex::encode(self.rights_id),
            hex::encode(self.title_key)
        )
    }
}

impl TitleKey {
    pub fn try_new<P: AsRef<Path>>(decrypted_tik_path: P) -> Result<TitleKey> {
        let mut title_key = TitleKey::default();
        let mut ticket = fs::File::open(decrypted_tik_path.as_ref())?;

        info!(tik = %decrypted_tik_path.as_ref().display(), "Reading ticket");

        ticket.seek(io::SeekFrom::Start(TitleKey::RIGHTS_ID_OFFSET as _))?;
        ticket.read_exact(&mut title_key.rights_id)?;

        ticket.seek(io::SeekFrom::Start(TitleKey::TITLE_KEY_OFFSET as _))?;
        ticket.read_exact(&mut title_key.title_key)?;
        debug!(
            title_key = %format!(
                "{}={}",
                hex::encode(title_key.rights_id),
                hex::encode(title_key.title_key)
            )
        );

        Ok(title_key)
    }
}
