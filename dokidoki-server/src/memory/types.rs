use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryType {
    Trivial,
    Normal,
    Important,
    Permanent,
}

impl MemoryType {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "trivial" => Some(Self::Trivial),
            "normal" => Some(Self::Normal),
            "important" => Some(Self::Important),
            "permanent" => Some(Self::Permanent),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trivial => "trivial",
            Self::Normal => "normal",
            Self::Important => "important",
            Self::Permanent => "permanent",
        }
    }

    pub fn default_ttl(&self) -> Option<Duration> {
        match self {
            Self::Trivial => Some(Duration::days(7)),
            Self::Normal => Some(Duration::days(30)),
            Self::Important => Some(Duration::days(90)),
            Self::Permanent => None,
        }
    }

    pub fn expires_at(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.default_ttl().map(|ttl| now + ttl)
    }
}

impl Default for MemoryType {
    fn default() -> Self {
        Self::Normal
    }
}
