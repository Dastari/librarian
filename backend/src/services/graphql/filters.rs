//! GraphQL filter input types for flexible querying
//!
//! These types enable ORM-style filtering on GraphQL queries with operators like:
//! - Eq, Ne (equals, not equals)
//! - Lt, Lte, Gt, Gte (comparisons)
//! - Contains, StartsWith, EndsWith (string matching)
//! - In, NotIn (list membership)
//! - Between (range queries)
//! - IsNull, IsNotNull (null checks)
//! - Similar (fuzzy text matching with Levenshtein distance)
//! - Date arithmetic (DaysAgo, DaysFromNow, Today, InPast, InFuture)
//!
//! All field names use PascalCase per the graphql-naming-convention.

use async_graphql::InputObject;

// ============================================================================
// Fuzzy/Similar Matching
// ============================================================================

/// Fuzzy matching filter for string similarity
#[derive(InputObject, Default, Clone, Debug)]
#[graphql(name = "SimilarFilter")]
pub struct SimilarFilter {
    /// The text to match against
    #[graphql(name = "Value")]
    pub value: String,
    /// Minimum similarity threshold (0.0-1.0, default 0.6)
    /// 1.0 = exact match, 0.0 = any match
    #[graphql(name = "Threshold")]
    pub threshold: Option<f64>,
}

impl SimilarFilter {
    /// Create a new similar filter with default threshold
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            threshold: None,
        }
    }

    /// Create a new similar filter with custom threshold
    pub fn with_threshold(value: impl Into<String>, threshold: f64) -> Self {
        Self {
            value: value.into(),
            threshold: Some(threshold),
        }
    }

    /// Get the threshold, defaulting to 0.6
    pub fn threshold_or_default(&self) -> f64 {
        self.threshold.unwrap_or(0.6)
    }
}

// ============================================================================
// Date Arithmetic
// ============================================================================

/// Relative date specification for date arithmetic
#[derive(InputObject, Default, Clone, Debug)]
#[graphql(name = "RelativeDate")]
pub struct RelativeDate {
    /// Number of days ago (positive = past)
    #[graphql(name = "DaysAgo")]
    pub days_ago: Option<i32>,
    /// Number of days from now (positive = future)
    #[graphql(name = "DaysFromNow")]
    pub days_from_now: Option<i32>,
    /// Use today's date
    #[graphql(name = "Today")]
    pub today: Option<bool>,
}

impl RelativeDate {
    /// Create a "days ago" relative date
    pub fn days_ago(days: i32) -> Self {
        Self {
            days_ago: Some(days),
            ..Default::default()
        }
    }

    /// Create a "days from now" relative date
    pub fn days_from_now(days: i32) -> Self {
        Self {
            days_from_now: Some(days),
            ..Default::default()
        }
    }

    /// Create a "today" relative date
    pub fn today() -> Self {
        Self {
            today: Some(true),
            ..Default::default()
        }
    }

    /// Convert to SQLite date expression
    pub fn to_sql_expr(&self) -> String {
        if self.today == Some(true) {
            return "date('now')".to_string();
        }
        if let Some(days) = self.days_ago {
            return format!("date('now', '-{} days')", days);
        }
        if let Some(days) = self.days_from_now {
            return format!("date('now', '+{} days')", days);
        }
        "date('now')".to_string()
    }
}

/// Filter for string fields
#[derive(InputObject, Default, Clone, Debug)]
#[graphql(name = "StringFilter")]
pub struct StringFilter {
    /// Equals
    #[graphql(name = "Eq")]
    pub eq: Option<String>,
    /// Not equals
    #[graphql(name = "Ne")]
    pub ne: Option<String>,
    /// Contains substring (case-insensitive)
    #[graphql(name = "Contains")]
    pub contains: Option<String>,
    /// Starts with
    #[graphql(name = "StartsWith")]
    pub starts_with: Option<String>,
    /// Ends with
    #[graphql(name = "EndsWith")]
    pub ends_with: Option<String>,
    /// In list
    #[graphql(name = "In")]
    pub in_list: Option<Vec<String>>,
    /// Not in list
    #[graphql(name = "NotIn")]
    pub not_in: Option<Vec<String>>,
    /// Is null
    #[graphql(name = "IsNull")]
    pub is_null: Option<bool>,
    /// Fuzzy/similar match with optional threshold (0.0-1.0, default 0.6)
    /// Uses normalized Levenshtein distance for scoring
    #[graphql(name = "Similar")]
    pub similar: Option<SimilarFilter>,
}

/// Filter for integer fields
#[derive(InputObject, Default, Clone, Debug)]
#[graphql(name = "IntFilter")]
pub struct IntFilter {
    /// Equals
    #[graphql(name = "Eq")]
    pub eq: Option<i32>,
    /// Not equals
    #[graphql(name = "Ne")]
    pub ne: Option<i32>,
    /// Less than
    #[graphql(name = "Lt")]
    pub lt: Option<i32>,
    /// Less than or equal
    #[graphql(name = "Lte")]
    pub lte: Option<i32>,
    /// Greater than
    #[graphql(name = "Gt")]
    pub gt: Option<i32>,
    /// Greater than or equal
    #[graphql(name = "Gte")]
    pub gte: Option<i32>,
    /// In list
    #[graphql(name = "In")]
    pub in_list: Option<Vec<i32>>,
    /// Not in list
    #[graphql(name = "NotIn")]
    pub not_in: Option<Vec<i32>>,
    /// Is null
    #[graphql(name = "IsNull")]
    pub is_null: Option<bool>,
}

/// Filter for boolean fields
#[derive(InputObject, Default, Clone, Debug)]
#[graphql(name = "BoolFilter")]
pub struct BoolFilter {
    /// Equals
    #[graphql(name = "Eq")]
    pub eq: Option<bool>,
    /// Not equals (opposite of eq)
    #[graphql(name = "Ne")]
    pub ne: Option<bool>,
    /// Is null
    #[graphql(name = "IsNull")]
    pub is_null: Option<bool>,
}

/// Date range for between queries
#[derive(InputObject, Default, Clone, Debug)]
#[graphql(name = "DateRange")]
pub struct DateRange {
    /// Start of range (inclusive)
    #[graphql(name = "Start")]
    pub start: Option<String>,
    /// End of range (inclusive)
    #[graphql(name = "End")]
    pub end: Option<String>,
}

/// Filter for date/timestamp fields
#[derive(InputObject, Default, Clone, Debug)]
#[graphql(name = "DateFilter")]
pub struct DateFilter {
    /// Equals
    #[graphql(name = "Eq")]
    pub eq: Option<String>,
    /// Not equals
    #[graphql(name = "Ne")]
    pub ne: Option<String>,
    /// Before (less than)
    #[graphql(name = "Lt")]
    pub lt: Option<String>,
    /// Before or on (less than or equal)
    #[graphql(name = "Lte")]
    pub lte: Option<String>,
    /// After (greater than)
    #[graphql(name = "Gt")]
    pub gt: Option<String>,
    /// After or on (greater than or equal)
    #[graphql(name = "Gte")]
    pub gte: Option<String>,
    /// Between two dates (inclusive)
    #[graphql(name = "Between")]
    pub between: Option<DateRange>,
    /// Is null
    #[graphql(name = "IsNull")]
    pub is_null: Option<bool>,

    // ========================================================================
    // Date Arithmetic Operators
    // ========================================================================
    /// In the past (before today)
    #[graphql(name = "InPast")]
    pub in_past: Option<bool>,
    /// In the future (after today)
    #[graphql(name = "InFuture")]
    pub in_future: Option<bool>,
    /// Is today
    #[graphql(name = "IsToday")]
    pub is_today: Option<bool>,
    /// Within the last N days (inclusive of today)
    #[graphql(name = "RecentDays")]
    pub recent_days: Option<i32>,
    /// Within the next N days (inclusive of today)
    #[graphql(name = "WithinDays")]
    pub within_days: Option<i32>,
    /// Greater than or equal to relative date
    #[graphql(name = "GteRelative")]
    pub gte_relative: Option<RelativeDate>,
    /// Less than or equal to relative date
    #[graphql(name = "LteRelative")]
    pub lte_relative: Option<RelativeDate>,
}

/// Order direction for sorting (legacy - prefer orm::OrderDirection)
#[derive(async_graphql::Enum, Copy, Clone, Debug, Default, Eq, PartialEq)]
#[graphql(name = "OrderDirection")]
pub enum OrderDirection {
    /// Ascending order (A-Z, 1-9, oldest-newest)
    #[default]
    #[graphql(name = "Asc")]
    Asc,
    /// Descending order (Z-A, 9-1, newest-oldest)
    #[graphql(name = "Desc")]
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
            && self.is_null.is_none()
            && self.similar.is_none()
    }

    // ========================================================================
    // Helper constructors for programmatic use
    // ========================================================================

    /// Create an equals filter
    pub fn eq(value: impl Into<String>) -> Self {
        Self {
            eq: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create a not-equals filter
    pub fn ne(value: impl Into<String>) -> Self {
        Self {
            ne: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create a contains filter (case-insensitive)
    pub fn contains(value: impl Into<String>) -> Self {
        Self {
            contains: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create a starts-with filter
    pub fn starts_with(value: impl Into<String>) -> Self {
        Self {
            starts_with: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create an ends-with filter
    pub fn ends_with(value: impl Into<String>) -> Self {
        Self {
            ends_with: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create an in-list filter
    pub fn in_list(values: Vec<String>) -> Self {
        Self {
            in_list: Some(values),
            ..Default::default()
        }
    }

    /// Create a not-in-list filter
    pub fn not_in(values: Vec<String>) -> Self {
        Self {
            not_in: Some(values),
            ..Default::default()
        }
    }

    /// Create an is-null filter
    pub fn is_null() -> Self {
        Self {
            is_null: Some(true),
            ..Default::default()
        }
    }

    /// Create an is-not-null filter
    pub fn is_not_null() -> Self {
        Self {
            is_null: Some(false),
            ..Default::default()
        }
    }

    /// Create a similar/fuzzy match filter
    pub fn similar(value: impl Into<String>) -> Self {
        Self {
            similar: Some(SimilarFilter::new(value)),
            ..Default::default()
        }
    }

    /// Create a similar/fuzzy match filter with custom threshold
    pub fn similar_with_threshold(value: impl Into<String>, threshold: f64) -> Self {
        Self {
            similar: Some(SimilarFilter::with_threshold(value, threshold)),
            ..Default::default()
        }
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
            && self.is_null.is_none()
    }

    /// Create an is-null filter
    pub fn is_null() -> Self {
        Self {
            is_null: Some(true),
            ..Default::default()
        }
    }

    /// Create an is-not-null filter
    pub fn is_not_null() -> Self {
        Self {
            is_null: Some(false),
            ..Default::default()
        }
    }

    // ========================================================================
    // Helper constructors for programmatic use
    // ========================================================================

    /// Create an equals filter
    pub fn eq(value: i32) -> Self {
        Self {
            eq: Some(value),
            ..Default::default()
        }
    }

    /// Create a not-equals filter
    pub fn ne(value: i32) -> Self {
        Self {
            ne: Some(value),
            ..Default::default()
        }
    }

    /// Create a less-than filter
    pub fn lt(value: i32) -> Self {
        Self {
            lt: Some(value),
            ..Default::default()
        }
    }

    /// Create a less-than-or-equal filter
    pub fn lte(value: i32) -> Self {
        Self {
            lte: Some(value),
            ..Default::default()
        }
    }

    /// Create a greater-than filter
    pub fn gt(value: i32) -> Self {
        Self {
            gt: Some(value),
            ..Default::default()
        }
    }

    /// Create a greater-than-or-equal filter
    pub fn gte(value: i32) -> Self {
        Self {
            gte: Some(value),
            ..Default::default()
        }
    }

    /// Create an in-list filter
    pub fn in_list(values: Vec<i32>) -> Self {
        Self {
            in_list: Some(values),
            ..Default::default()
        }
    }

    /// Create a not-in-list filter
    pub fn not_in(values: Vec<i32>) -> Self {
        Self {
            not_in: Some(values),
            ..Default::default()
        }
    }

    /// Create a between filter (inclusive)
    pub fn between(min: i32, max: i32) -> Self {
        Self {
            gte: Some(min),
            lte: Some(max),
            ..Default::default()
        }
    }
}

impl BoolFilter {
    /// Check if filter has any conditions
    pub fn is_empty(&self) -> bool {
        self.eq.is_none() && self.ne.is_none() && self.is_null.is_none()
    }

    // ========================================================================
    // Helper constructors for programmatic use
    // ========================================================================

    /// Create a filter for true values
    pub fn is_true() -> Self {
        Self {
            eq: Some(true),
            ..Default::default()
        }
    }

    /// Create a filter for false values
    pub fn is_false() -> Self {
        Self {
            eq: Some(false),
            ..Default::default()
        }
    }

    /// Create an equals filter
    pub fn eq(value: bool) -> Self {
        Self {
            eq: Some(value),
            ..Default::default()
        }
    }

    /// Create an is-null filter
    pub fn is_null() -> Self {
        Self {
            is_null: Some(true),
            ..Default::default()
        }
    }

    /// Create an is-not-null filter
    pub fn is_not_null() -> Self {
        Self {
            is_null: Some(false),
            ..Default::default()
        }
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
            && self.is_null.is_none()
            && self.in_past.is_none()
            && self.in_future.is_none()
            && self.is_today.is_none()
            && self.recent_days.is_none()
            && self.within_days.is_none()
            && self.gte_relative.is_none()
            && self.lte_relative.is_none()
    }

    // ========================================================================
    // Helper constructors for programmatic use
    // ========================================================================

    /// Create an equals filter
    pub fn eq(value: impl Into<String>) -> Self {
        Self {
            eq: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create a before (less than) filter
    pub fn before(value: impl Into<String>) -> Self {
        Self {
            lt: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create an after (greater than) filter
    pub fn after(value: impl Into<String>) -> Self {
        Self {
            gt: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create a between filter
    pub fn between(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            between: Some(DateRange {
                start: Some(start.into()),
                end: Some(end.into()),
            }),
            ..Default::default()
        }
    }

    /// Create a greater-than-or-equal filter
    pub fn gte(value: impl Into<String>) -> Self {
        Self {
            gte: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create a less-than-or-equal filter
    pub fn lte(value: impl Into<String>) -> Self {
        Self {
            lte: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create an is-null filter
    pub fn is_null() -> Self {
        Self {
            is_null: Some(true),
            ..Default::default()
        }
    }

    /// Create an is-not-null filter
    pub fn is_not_null() -> Self {
        Self {
            is_null: Some(false),
            ..Default::default()
        }
    }

    /// Create a filter for dates in the past
    pub fn in_past() -> Self {
        Self {
            in_past: Some(true),
            ..Default::default()
        }
    }

    /// Create a filter for dates in the future
    pub fn in_future() -> Self {
        Self {
            in_future: Some(true),
            ..Default::default()
        }
    }

    /// Create a filter for today
    pub fn today() -> Self {
        Self {
            is_today: Some(true),
            ..Default::default()
        }
    }

    /// Create a filter for dates within the last N days
    pub fn recent_days(days: i32) -> Self {
        Self {
            recent_days: Some(days),
            ..Default::default()
        }
    }

    /// Create a filter for dates within the next N days
    pub fn within_days(days: i32) -> Self {
        Self {
            within_days: Some(days),
            ..Default::default()
        }
    }
}
