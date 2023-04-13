pub mod nca;
pub mod nsp;
pub mod ticket;

// Yes....this is vfs....

// TODO: add nacp handling for ApplicationName and ApplicationVersion
// TODO: npdm handling for nsp type? (Patch or Application) - helpful for differentiating

// No. of hexadecimal characters
pub const PROGRAMID_LEN: u8 = 16;
