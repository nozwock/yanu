use super::rom::Nsp;
use anyhow::Result;
use std::path::Path;

/// `title_key_path` is the Path where TitleKeys will be stored (optional).
pub fn patch_nsp_with_update<P>(
    base: &mut Nsp,
    update: &mut Nsp,
    title_key_path: Option<P>,
) -> Result<Nsp>
where
    P: AsRef<Path>,
{
    if base.title_key.is_none() {
        base.extract_title_key()?; // might need a change in future!? (err handling)
    }
    if update.title_key.is_none() {
        update.extract_title_key()?;
    }

    todo!();
}
