use eyre::{eyre, Report};
use std::fmt::Display;

pub struct MultiReport {
    pub errs: Vec<Report>,
}

impl Display for MultiReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.join("\n"))
    }
}

impl MultiReport {
    pub fn new<E>(errs: E) -> Self
    where
        E: IntoIterator<Item = Report>,
    {
        Self {
            errs: errs.into_iter().collect(),
        }
    }
    pub fn join(&self, sep: &str) -> Report {
        let mut err_str = self
            .errs
            .iter()
            .map(|report| report.to_string())
            .collect::<Vec<_>>()
            .join(sep);
        if self.errs.len() > 1 {
            err_str.insert(0, '\n');
        }
        eyre!("{}", err_str)
    }
}
