//! Stream categories (Xtream calls them `category_id` / `category_name`).
//!
//! The id keeps the provider's own category identifier so streams can reference
//! their category without us inventing a parallel numbering scheme.

use serde::{Deserialize, Serialize};

/// A provider's category identifier, kept as-is so streams can reference it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CategoryId(pub String);

/// A normalized stream category, identical regardless of source type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        // Categories cross the command boundary, so serde must round-trip cleanly.
        let category = Category {
            id: CategoryId("5".to_string()),
            name: "Sports".to_string(),
        };
        let json = serde_json::to_string(&category).unwrap();
        let back: Category = serde_json::from_str(&json).unwrap();
        assert_eq!(category, back);
    }
}
