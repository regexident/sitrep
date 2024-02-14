//! A progress event.

use std::str::FromStr;

const MIN_PRIORITY_LEVEL_KEY: &str = "SITREP_PRIO";

/// A message's priority level.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Debug)]
pub enum PriorityLevel {
    /// A message at the "trace" level.
    #[default]
    Trace = 0,
    /// A message at the "debug" level.
    Debug = 1,
    /// A message at the "info" level.
    Info = 2,
    /// A message at the "warn" level.
    Warn = 3,
    /// A message at the "error" level.
    Error = 4,
}

impl PriorityLevel {
    /// The minimum available priority level (`Self::Trace`).
    pub const MIN: Self = Self::Trace;

    /// The maximum available priority level (`Self::Error`).
    pub const MAX: Self = Self::Error;

    /// All available priority level in increasing order.
    pub const ALL: [Self; 5] = [
        Self::Trace,
        Self::Debug,
        Self::Info,
        Self::Warn,
        Self::Error,
    ];

    pub(crate) fn from_env() -> Option<Self> {
        use std::str::FromStr as _;

        let Ok(level) = std::env::var(MIN_PRIORITY_LEVEL_KEY) else {
            return None;
        };

        match EnvPriorityLevel::from_str(&level) {
            Ok(EnvPriorityLevel(min_level)) => Some(min_level),
            Err(err) => panic!(
                "{}",
                format!(
                    "Unrecognized value for env key {MIN_PRIORITY_LEVEL_KEY:?}: {value:?}",
                    value = err.unknown
                )
            ),
        }
    }
}

pub(crate) struct EnvPriorityLevelError {
    pub(crate) unknown: String,
}

pub(crate) struct EnvPriorityLevel(pub(crate) PriorityLevel);

impl FromStr for EnvPriorityLevel {
    type Err = EnvPriorityLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let string = s.to_lowercase();
        match string.as_str() {
            "trace" => Ok(Self(PriorityLevel::Trace)),
            "debug" => Ok(Self(PriorityLevel::Debug)),
            "info" => Ok(Self(PriorityLevel::Info)),
            "warn" => Ok(Self(PriorityLevel::Warn)),
            "error" => Ok(Self(PriorityLevel::Error)),
            _ => Err(EnvPriorityLevelError { unknown: string }),
        }
    }
}
