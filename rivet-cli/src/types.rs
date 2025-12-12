//! Common types used across CLI modules

use uuid::Uuid;

/// Identifier that can be either a full UUID or an unambiguous prefix
#[derive(Debug, Clone)]
pub enum IdOrPrefix {
    /// Full UUID
    Full(Uuid),
    /// Prefix that should uniquely identify a resource
    Prefix(String),
}

impl IdOrPrefix {
    /// Parse a string into an IdOrPrefix
    ///
    /// Attempts to parse as a full UUID first, otherwise treats as a prefix
    pub fn parse(input: &str) -> Self {
        if let Ok(uuid) = Uuid::parse_str(input) {
            IdOrPrefix::Full(uuid)
        } else {
            IdOrPrefix::Prefix(input.to_string())
        }
    }

    /// Get the UUID if this is a full ID
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            IdOrPrefix::Full(uuid) => Some(*uuid),
            IdOrPrefix::Prefix(_) => None,
        }
    }

    /// Get the prefix string
    pub fn as_str(&self) -> String {
        match self {
            IdOrPrefix::Full(uuid) => uuid.to_string(),
            IdOrPrefix::Prefix(prefix) => prefix.clone(),
        }
    }
}

impl std::fmt::Display for IdOrPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdOrPrefix::Full(uuid) => write!(f, "{}", uuid),
            IdOrPrefix::Prefix(prefix) => write!(f, "{}", prefix),
        }
    }
}

impl From<Uuid> for IdOrPrefix {
    fn from(uuid: Uuid) -> Self {
        IdOrPrefix::Full(uuid)
    }
}

impl From<String> for IdOrPrefix {
    fn from(s: String) -> Self {
        IdOrPrefix::parse(&s)
    }
}

impl From<&str> for IdOrPrefix {
    fn from(s: &str) -> Self {
        IdOrPrefix::parse(s)
    }
}
