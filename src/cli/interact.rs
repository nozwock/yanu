use core::fmt;

#[derive(Debug)]
pub enum CliOptions {
    BasePkg,
    UpdatePkg,
}

impl fmt::Display for CliOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliOptions::BasePkg => write!(f, "Select base package"),
            CliOptions::UpdatePkg => write!(f, "Select update package"),
        }
    }
}
