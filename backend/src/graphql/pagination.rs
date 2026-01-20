//! Cursor-based pagination types for GraphQL
//!
//! Implements the Relay Connection specification for consistent pagination
//! across all list queries.
//!
//! Usage: Use the `define_connection!` macro to create type-specific connections.

use async_graphql::SimpleObject;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Information about pagination in a connection
#[derive(SimpleObject, Debug, Clone, Default)]
pub struct PageInfo {
    /// When paginating forwards, are there more items?
    pub has_next_page: bool,
    /// When paginating backwards, are there more items?
    pub has_previous_page: bool,
    /// Cursor of the first item in this page
    pub start_cursor: Option<String>,
    /// Cursor of the last item in this page
    pub end_cursor: Option<String>,
    /// Total count of items (if available)
    pub total_count: Option<i64>,
}

/// An edge in a connection, containing a node and cursor (internal use)
#[derive(Debug, Clone)]
pub struct Edge<T> {
    /// The item at the end of the edge
    pub node: T,
    /// A cursor for pagination
    pub cursor: String,
}

/// A paginated connection result (internal use)
#[derive(Debug, Clone)]
pub struct Connection<T> {
    /// The edges in this connection
    pub edges: Vec<Edge<T>>,
    /// Pagination information
    pub page_info: PageInfo,
}

/// Macro to define a GraphQL connection type for a specific entity
///
/// Usage:
/// ```ignore
/// define_connection!(MovieConnection, MovieEdge, Movie);
/// ```
#[macro_export]
macro_rules! define_connection {
    ($conn_name:ident, $edge_name:ident, $node_type:ty) => {
        /// Edge containing a node and cursor
        #[derive(async_graphql::SimpleObject, Debug, Clone)]
        pub struct $edge_name {
            /// The item at the end of the edge
            pub node: $node_type,
            /// A cursor for pagination
            pub cursor: String,
        }

        /// Connection containing edges and page info
        #[derive(async_graphql::SimpleObject, Debug, Clone)]
        pub struct $conn_name {
            /// The edges in this connection
            pub edges: Vec<$edge_name>,
            /// Pagination information
            pub page_info: $crate::graphql::pagination::PageInfo,
        }

        impl $conn_name {
            /// Create from a generic Connection
            pub fn from_connection(
                conn: $crate::graphql::pagination::Connection<$node_type>,
            ) -> Self {
                Self {
                    edges: conn
                        .edges
                        .into_iter()
                        .map(|e| $edge_name {
                            node: e.node,
                            cursor: e.cursor,
                        })
                        .collect(),
                    page_info: conn.page_info,
                }
            }
        }
    };
}

impl<T> Connection<T> {
    /// Create an empty connection
    pub fn empty() -> Self {
        Self {
            edges: Vec::new(),
            page_info: PageInfo {
                has_next_page: false,
                has_previous_page: false,
                start_cursor: None,
                end_cursor: None,
                total_count: Some(0),
            },
        }
    }

    /// Create a connection from a list of items
    ///
    /// # Arguments
    /// * `items` - The items to include in this page
    /// * `offset` - The offset of the first item (for cursor generation)
    /// * `limit` - The requested limit (to determine if there are more pages)
    /// * `total` - Total count of items matching the query
    pub fn from_items(items: Vec<T>, offset: i64, _limit: i64, total: i64) -> Self
    where
        T: Clone,
    {
        let has_next_page = (offset + items.len() as i64) < total;
        let has_previous_page = offset > 0;

        let edges: Vec<Edge<T>> = items
            .into_iter()
            .enumerate()
            .map(|(i, node)| Edge {
                cursor: encode_cursor(offset + i as i64),
                node,
            })
            .collect();

        let page_info = PageInfo {
            has_next_page,
            has_previous_page,
            start_cursor: edges.first().map(|e| e.cursor.clone()),
            end_cursor: edges.last().map(|e| e.cursor.clone()),
            total_count: Some(total),
        };

        Self { edges, page_info }
    }
}

/// Encode an offset as a cursor string
pub fn encode_cursor(offset: i64) -> String {
    BASE64.encode(format!("cursor:{}", offset))
}

/// Decode a cursor string to an offset
pub fn decode_cursor(cursor: &str) -> Result<i64, &'static str> {
    let decoded = BASE64.decode(cursor).map_err(|_| "invalid cursor format")?;

    let s = String::from_utf8(decoded).map_err(|_| "invalid cursor encoding")?;

    if !s.starts_with("cursor:") {
        return Err("invalid cursor prefix");
    }

    s[7..].parse().map_err(|_| "invalid cursor value")
}

/// Parse pagination arguments into offset and limit
pub fn parse_pagination_args(
    first: Option<i32>,
    after: Option<String>,
) -> Result<(i64, i64), &'static str> {
    let limit = first.unwrap_or(25).min(100) as i64;

    let offset = if let Some(cursor) = after {
        decode_cursor(&cursor)? + 1 // Start after the cursor
    } else {
        0
    };

    Ok((offset, limit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_roundtrip() {
        for offset in [0, 1, 100, 999999] {
            let cursor = encode_cursor(offset);
            let decoded = decode_cursor(&cursor).unwrap();
            assert_eq!(offset, decoded);
        }
    }

    #[test]
    fn test_parse_pagination_default() {
        let (offset, limit) = parse_pagination_args(None, None).unwrap();
        assert_eq!(offset, 0);
        assert_eq!(limit, 25);
    }

    #[test]
    fn test_parse_pagination_with_limit() {
        let (offset, limit) = parse_pagination_args(Some(50), None).unwrap();
        assert_eq!(offset, 0);
        assert_eq!(limit, 50);
    }

    #[test]
    fn test_parse_pagination_max_limit() {
        let (offset, limit) = parse_pagination_args(Some(1000), None).unwrap();
        assert_eq!(offset, 0);
        assert_eq!(limit, 100); // Capped at 100
    }

    #[test]
    fn test_parse_pagination_with_cursor() {
        let cursor = encode_cursor(10);
        let (offset, limit) = parse_pagination_args(Some(25), Some(cursor)).unwrap();
        assert_eq!(offset, 11); // After cursor at offset 10
        assert_eq!(limit, 25);
    }
}
