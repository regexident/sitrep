//! A progress event.

use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicU8, Ordering},
        OnceLock,
    },
};

const MIN_PRIORITY_LEVEL_KEY: &str = "SITREP_PRIO";

pub(crate) fn global_min_priority_level() -> PriorityLevel {
    static MIN_PRIORITY_LEVEL: OnceLock<PriorityLevel> = OnceLock::new();

    *MIN_PRIORITY_LEVEL.get_or_init(|| PriorityLevel::from_env().unwrap_or(PriorityLevel::MIN))
}

/// A message's priority level.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Debug)]
#[repr(u8)]
pub enum PriorityLevel {
    /// A message at the "trace" level.
    #[default]
    Trace = 1,
    /// A message at the "debug" level.
    Debug = 2,
    /// A message at the "info" level.
    Info = 3,
    /// A message at the "warn" level.
    Warn = 4,
    /// A message at the "error" level.
    Error = 5,
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
            Err(_err) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    env_key = MIN_PRIORITY_LEVEL_KEY,
                    value = ?_err.unknown,
                    "Unrecognized value for environment variable. Using default."
                );
                None
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct EnvPriorityLevelError {
    #[allow(dead_code)]
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
            _ => Err(Self::Err { unknown: string }),
        }
    }
}

#[derive(Debug)]
pub(crate) struct PriorityLevelReprError {
    #[allow(dead_code)]
    pub(crate) unknown: u8,
}

pub(crate) struct PriorityLevelRepr(pub(crate) PriorityLevel);

impl TryFrom<u8> for PriorityLevelRepr {
    type Error = PriorityLevelReprError;

    fn try_from(repr: u8) -> Result<Self, Self::Error> {
        use PriorityLevel::*;

        match repr {
            x if x == Trace as u8 => Ok(Self(Trace)),
            x if x == Debug as u8 => Ok(Self(Debug)),
            x if x == Info as u8 => Ok(Self(Info)),
            x if x == Warn as u8 => Ok(Self(Warn)),
            x if x == Error as u8 => Ok(Self(Error)),
            unknown => Err(Self::Error { unknown }),
        }
    }
}

pub(crate) struct AtomicPriorityLevel(pub(crate) AtomicU8);

impl From<PriorityLevel> for AtomicPriorityLevel {
    fn from(level: PriorityLevel) -> Self {
        Self(AtomicU8::from(level as u8))
    }
}

impl AtomicPriorityLevel {
    pub(crate) fn load(&self, order: Ordering) -> Option<PriorityLevel> {
        let repr = self.0.load(order);

        if repr == 0 {
            return None;
        }

        Some(PriorityLevelRepr::try_from(repr).unwrap().0)
    }

    pub(crate) fn store(&self, level: Option<PriorityLevel>, order: Ordering) {
        let repr = level.map(|level| level as u8).unwrap_or(0);

        self.0.store(repr, order)
    }
}
