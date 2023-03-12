//! Improper cheap implementation

use eyre::Result;
use fs_err as fs;
use std::{
    fmt,
    io::{self, Read, Seek},
    path::Path,
};
use tracing::{debug, info};

enum TicketData {
    TitleId = 0x2a0,
    TitleKey = 0x180,
}

#[derive(Debug, Default, Clone)]
pub struct TitleKey {
    title_id: [u8; 16],
    key: [u8; 16],
}

impl fmt::Display for TitleKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}={}",
            hex::encode(self.title_id),
            hex::encode(self.key)
        )
    }
}

pub fn get_title_key<P: AsRef<Path>>(tik_path: P) -> Result<TitleKey> {
    let mut title_key = TitleKey::default();
    let mut ticket = fs::File::open(tik_path.as_ref())?;

    info!(tik = ?tik_path.as_ref(), "Reading ticket");

    ticket.seek(io::SeekFrom::Start(TicketData::TitleId as _))?;
    ticket.read_exact(&mut title_key.title_id)?;

    ticket.seek(io::SeekFrom::Start(TicketData::TitleKey as _))?;
    ticket.read_exact(&mut title_key.key)?;
    debug!(
        title_key = ?format!(
            "{}={}",
            hex::encode(title_key.title_id),
            hex::encode(title_key.key)
        )
    );

    Ok(title_key)
}
