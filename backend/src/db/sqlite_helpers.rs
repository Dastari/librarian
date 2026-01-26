//! SQLite helper utilities for type conversion
//!
//! SQLite doesn't natively support UUIDs, arrays, or JSONB like PostgreSQL.
//! This module provides utilities to convert between Rust types and SQLite-compatible formats.

use anyhow::{Result, anyhow};
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;

// ============================================================================
// UUID Helpers
// ============================================================================

/// Convert a UUID to a SQLite-compatible string
#[inline]
pub fn uuid_to_str(id: Uuid) -> String {
    id.to_string()
}

/// Convert a UUID reference to a SQLite-compatible string
#[inline]
pub fn uuid_ref_to_str(id: &Uuid) -> String {
    id.to_string()
}

/// Parse a SQLite string back to a UUID
#[inline]
pub fn str_to_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| anyhow!("Invalid UUID '{}': {}", s, e))
}

/// Parse an optional SQLite string to an optional UUID
#[inline]
pub fn str_to_uuid_opt(s: Option<&str>) -> Result<Option<Uuid>> {
    match s {
        Some(s) => Ok(Some(str_to_uuid(s)?)),
        None => Ok(None),
    }
}

// ============================================================================
// Array/Vec Helpers (stored as JSON strings in SQLite)
// ============================================================================

/// Serialize a Vec to a JSON string for SQLite storage
#[inline]
pub fn vec_to_json<T: Serialize>(v: &[T]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string())
}

/// Deserialize a JSON string from SQLite to a Vec
#[inline]
pub fn json_to_vec<T: DeserializeOwned>(s: &str) -> Vec<T> {
    serde_json::from_str(s).unwrap_or_default()
}

/// Deserialize an optional JSON string to a Vec (returns empty vec if None or invalid)
#[inline]
pub fn json_to_vec_opt<T: DeserializeOwned>(s: Option<&str>) -> Vec<T> {
    match s {
        Some(s) => json_to_vec(s),
        None => Vec::new(),
    }
}

/// Serialize an optional Vec to JSON string (None becomes "[]")
#[inline]
pub fn vec_opt_to_json<T: Serialize>(v: Option<&[T]>) -> String {
    match v {
        Some(v) => vec_to_json(v),
        None => "[]".to_string(),
    }
}

// ============================================================================
// JSONB/JSON Helpers (stored as TEXT in SQLite)
// ============================================================================

/// Serialize any serializable value to a JSON string
#[inline]
pub fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

/// Deserialize a JSON string to a value
#[inline]
pub fn from_json<T: DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_str(s).map_err(|e| anyhow!("JSON parse error: {}", e))
}

/// Deserialize an optional JSON string
#[inline]
pub fn from_json_opt<T: DeserializeOwned>(s: Option<&str>) -> Result<Option<T>> {
    match s {
        Some(s) if !s.is_empty() && s != "null" => Ok(Some(from_json(s)?)),
        _ => Ok(None),
    }
}

// ============================================================================
// Timestamp Helpers (stored as ISO8601 TEXT in SQLite)
// ============================================================================

use chrono::{DateTime, Utc};

/// Get current UTC timestamp as ISO8601 string for SQLite
#[inline]
pub fn now_iso8601() -> String {
    Utc::now().to_rfc3339()
}

/// Convert a chrono DateTime to ISO8601 string
#[inline]
pub fn datetime_to_str(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// Parse an ISO8601 string to DateTime
#[inline]
pub fn str_to_datetime(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            // Try parsing SQLite's datetime() format: "YYYY-MM-DD HH:MM:SS"
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| ndt.and_utc())
                .map_err(|e| anyhow!("Invalid datetime '{}': {}", s, e))
        })
}

/// Parse an optional datetime string
#[inline]
pub fn str_to_datetime_opt(s: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    match s {
        Some(s) if !s.is_empty() => Ok(Some(str_to_datetime(s)?)),
        _ => Ok(None),
    }
}

// ============================================================================
// Boolean Helpers (SQLite uses 0/1 integers)
// ============================================================================

/// Convert bool to SQLite integer (0 or 1)
#[inline]
pub fn bool_to_int(b: bool) -> i32 {
    if b { 1 } else { 0 }
}

/// Convert SQLite integer to bool
#[inline]
pub fn int_to_bool(i: i32) -> bool {
    i != 0
}

// ============================================================================
// Query Building Helpers
// ============================================================================

/// Build a SQL fragment to check if a value exists in a JSON array column
/// Usage: format!("EXISTS (SELECT 1 FROM json_each({}) WHERE value = ?)", column)
pub fn json_array_contains_sql(column: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM json_each({}) WHERE value = ?)",
        column
    )
}

/// Build a SQL fragment to check if any value from a list exists in a JSON array column
pub fn json_array_overlaps_sql(column: &str, placeholder_count: usize) -> String {
    if placeholder_count == 0 {
        return "1=0".to_string(); // Always false for empty list
    }

    let placeholders: Vec<&str> = (0..placeholder_count).map(|_| "?").collect();
    format!(
        "EXISTS (SELECT 1 FROM json_each({}) WHERE value IN ({}))",
        column,
        placeholders.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_uuid_roundtrip() {
        let id = Uuid::new_v4();
        let s = uuid_to_str(id);
        let parsed = str_to_uuid(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_vec_json_roundtrip() {
        let v = vec!["hello".to_string(), "world".to_string()];
        let json = vec_to_json(&v);
        let parsed: Vec<String> = json_to_vec(&json);
        assert_eq!(v, parsed);
    }

    #[test]
    fn test_empty_vec() {
        let v: Vec<String> = vec![];
        let json = vec_to_json(&v);
        assert_eq!(json, "[]");
        let parsed: Vec<String> = json_to_vec(&json);
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_datetime_roundtrip() {
        let dt = Utc::now();
        let s = datetime_to_str(dt);
        let parsed = str_to_datetime(&s).unwrap();
        // Compare to second precision (rfc3339 might have slight differences)
        assert_eq!(dt.timestamp(), parsed.timestamp());
    }

    #[test]
    fn test_sqlite_datetime_format() {
        let s = "2024-01-15 10:30:45";
        let parsed = str_to_datetime(s).unwrap();
        assert_eq!(parsed.year(), 2024);
        assert_eq!(parsed.month(), 1);
        assert_eq!(parsed.day(), 15);
    }

    #[test]
    fn test_bool_conversion() {
        assert_eq!(bool_to_int(true), 1);
        assert_eq!(bool_to_int(false), 0);
        assert!(int_to_bool(1));
        assert!(int_to_bool(42)); // Any non-zero is true
        assert!(!int_to_bool(0));
    }

    #[test]
    fn test_json_array_contains_sql() {
        let sql = json_array_contains_sql("genres");
        assert_eq!(
            sql,
            "EXISTS (SELECT 1 FROM json_each(genres) WHERE value = ?)"
        );
    }
}
