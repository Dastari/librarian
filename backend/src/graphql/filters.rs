//! GraphQL filter input types for flexible querying
//!
//! These types enable ORM-style filtering on GraphQL queries with operators like:
//! - eq, ne (equals, not equals)
//! - lt, lte, gt, gte (comparisons)
//! - contains, startsWith, endsWith (string matching)
//! - in, notIn (list membership)
//! - between (range queries)

use async_graphql::InputObject;

/// Filter for string fields
#[derive(InputObject, Default, Clone, Debug)]
pub struct StringFilter {
    /// Equals
    pub eq: Option<String>,
    /// Not equals
    pub ne: Option<String>,
    /// Contains substring (case-insensitive)
    pub contains: Option<String>,
    /// Starts with
    pub starts_with: Option<String>,
    /// Ends with
    pub ends_with: Option<String>,
    /// In list
    #[graphql(name = "in")]
    pub in_list: Option<Vec<String>>,
    /// Not in list
    pub not_in: Option<Vec<String>>,
}

/// Filter for integer fields
#[derive(InputObject, Default, Clone, Debug)]
pub struct IntFilter {
    /// Equals
    pub eq: Option<i32>,
    /// Not equals
    pub ne: Option<i32>,
    /// Less than
    pub lt: Option<i32>,
    /// Less than or equal
    pub lte: Option<i32>,
    /// Greater than
    pub gt: Option<i32>,
    /// Greater than or equal
    pub gte: Option<i32>,
    /// In list
    #[graphql(name = "in")]
    pub in_list: Option<Vec<i32>>,
    /// Not in list
    pub not_in: Option<Vec<i32>>,
}

/// Filter for boolean fields
#[derive(InputObject, Default, Clone, Debug)]
pub struct BoolFilter {
    /// Equals
    pub eq: Option<bool>,
    /// Not equals (opposite of eq)
    pub ne: Option<bool>,
}

/// Date range for between queries
#[derive(InputObject, Default, Clone, Debug)]
pub struct DateRange {
    /// Start of range (inclusive)
    pub start: Option<String>,
    /// End of range (inclusive)
    pub end: Option<String>,
}

/// Filter for date/timestamp fields
#[derive(InputObject, Default, Clone, Debug)]
pub struct DateFilter {
    /// Equals
    pub eq: Option<String>,
    /// Not equals
    pub ne: Option<String>,
    /// Before (less than)
    pub lt: Option<String>,
    /// Before or on (less than or equal)
    pub lte: Option<String>,
    /// After (greater than)
    pub gt: Option<String>,
    /// After or on (greater than or equal)
    pub gte: Option<String>,
    /// Between two dates (inclusive)
    pub between: Option<DateRange>,
}

/// Order direction for sorting
#[derive(async_graphql::Enum, Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum OrderDirection {
    /// Ascending order (A-Z, 1-9, oldest-newest)
    #[default]
    Asc,
    /// Descending order (Z-A, 9-1, newest-oldest)
    Desc,
}

impl OrderDirection {
    /// Convert to SQL order string
    pub fn to_sql(&self) -> &'static str {
        match self {
            OrderDirection::Asc => "ASC",
            OrderDirection::Desc => "DESC",
        }
    }
}

// ============================================================================
// SQL Generation Utilities
// ============================================================================

impl StringFilter {
    /// Check if filter has any conditions
    pub fn is_empty(&self) -> bool {
        self.eq.is_none()
            && self.ne.is_none()
            && self.contains.is_none()
            && self.starts_with.is_none()
            && self.ends_with.is_none()
            && self.in_list.as_ref().map_or(true, |v| v.is_empty())
            && self.not_in.as_ref().map_or(true, |v| v.is_empty())
    }
}

impl IntFilter {
    /// Check if filter has any conditions
    pub fn is_empty(&self) -> bool {
        self.eq.is_none()
            && self.ne.is_none()
            && self.lt.is_none()
            && self.lte.is_none()
            && self.gt.is_none()
            && self.gte.is_none()
            && self.in_list.as_ref().map_or(true, |v| v.is_empty())
            && self.not_in.as_ref().map_or(true, |v| v.is_empty())
    }
}

impl BoolFilter {
    /// Check if filter has any conditions
    pub fn is_empty(&self) -> bool {
        self.eq.is_none() && self.ne.is_none()
    }
}

impl DateFilter {
    /// Check if filter has any conditions
    pub fn is_empty(&self) -> bool {
        self.eq.is_none()
            && self.ne.is_none()
            && self.lt.is_none()
            && self.lte.is_none()
            && self.gt.is_none()
            && self.gte.is_none()
            && self.between.is_none()
    }
}
