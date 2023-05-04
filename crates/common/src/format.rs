use std::{fmt, time::Duration};

const SECOND: Duration = Duration::from_secs(1);
const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(60 * 60);
const DAY: Duration = Duration::from_secs(24 * 60 * 60);
const WEEK: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const YEAR: Duration = Duration::from_secs(365 * 24 * 60 * 60);

/// Stolen from `indicatif`\
/// https://github.com/console-rs/indicatif/blob/main/src/format.rs
pub struct HumanDuration(pub Duration);

const UNITS: &[(Duration, &str, &str)] = &[
    (YEAR, "year", "y"),
    (WEEK, "week", "w"),
    (DAY, "day", "d"),
    (HOUR, "hour", "h"),
    (MINUTE, "minute", "m"),
    (SECOND, "second", "s"),
];

impl fmt::Display for HumanDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut idx = 0;
        for (i, &(cur, _, _)) in UNITS.iter().enumerate() {
            idx = i;
            match UNITS.get(i + 1) {
                Some(&next) if self.0 + next.0 / 2 >= cur + cur / 2 => break,
                _ => continue,
            }
        }

        let (unit, name, alt) = UNITS[idx];
        // FIXME when `div_duration_f64` is stable
        let mut t = (self.0.as_secs_f64() / unit.as_secs_f64()).round() as usize;
        if idx < UNITS.len() - 1 {
            t = Ord::max(t, 2);
        }

        match (f.alternate(), t) {
            (true, _) => write!(f, "{t}{alt}"),
            (false, 1) => write!(f, "{t} {name}"),
            (false, _) => write!(f, "{t} {name}s"),
        }
    }
}
