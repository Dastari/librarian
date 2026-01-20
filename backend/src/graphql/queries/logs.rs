use super::prelude::*;

#[derive(Default)]
pub struct LogQueries;

#[Object]
impl LogQueries {
    /// Get logs with optional filtering and pagination
    async fn logs(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Filter options")] filter: Option<LogFilterInput>,
        #[graphql(desc = "Sort order")] order_by: Option<LogOrderByInput>,
        #[graphql(default = 50, desc = "Number of logs to return")] limit: i32,
        #[graphql(default = 0, desc = "Offset for pagination")] offset: i32,
    ) -> Result<PaginatedLogResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let log_filter = filter
            .map(|f| {
                let levels = f.levels.map(|ls| {
                    ls.into_iter()
                        .map(|l| match l {
                            LogLevel::Trace => "TRACE".to_string(),
                            LogLevel::Debug => "DEBUG".to_string(),
                            LogLevel::Info => "INFO".to_string(),
                            LogLevel::Warn => "WARN".to_string(),
                            LogLevel::Error => "ERROR".to_string(),
                        })
                        .collect()
                });

                let from_timestamp = f.from_timestamp.and_then(|s| {
                    time::OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
                        .ok()
                });

                let to_timestamp = f.to_timestamp.and_then(|s| {
                    time::OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
                        .ok()
                });

                LogFilter {
                    levels,
                    target: f.target,
                    keyword: f.keyword,
                    from_timestamp,
                    to_timestamp,
                }
            })
            .unwrap_or_default();

        // Convert order_by to database format
        let order = order_by.map(|o| {
            use crate::db::logs::LogOrderBy;
            use crate::graphql::filters::OrderDirection;

            let field = match o.field.unwrap_or_default() {
                LogSortField::TIMESTAMP => "timestamp",
                LogSortField::LEVEL => "level",
                LogSortField::TARGET => "target",
            };

            let direction = match o.direction.unwrap_or(OrderDirection::Desc) {
                OrderDirection::Asc => "ASC",
                OrderDirection::Desc => "DESC",
            };
            LogOrderBy {
                field: field.to_string(),
                direction: direction.to_string(),
            }
        });

        let result = db
            .logs()
            .list(log_filter, order, limit as i64, offset as i64)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let next_cursor = result.logs.last().map(|l| {
            l.timestamp
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default()
        });

        Ok(PaginatedLogResult {
            logs: result
                .logs
                .into_iter()
                .map(|r| LogEntry {
                    id: r.id.to_string(),
                    timestamp: r
                        .timestamp
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                    level: LogLevel::from(r.level.as_str()),
                    target: r.target,
                    message: r.message,
                    fields: r.fields,
                    span_name: r.span_name,
                })
                .collect(),
            total_count: result.total_count,
            has_more: result.has_more,
            next_cursor,
        })
    }

    /// Get distinct log targets for filtering
    async fn log_targets(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50, desc = "Maximum number of targets to return")] limit: i32,
    ) -> Result<Vec<String>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let targets = db
            .logs()
            .get_distinct_targets(limit as i64)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(targets)
    }

    /// Get log statistics by level
    async fn log_stats(&self, ctx: &Context<'_>) -> Result<LogStats> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let counts = db
            .logs()
            .get_counts_by_level()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut stats = LogStats {
            trace_count: 0,
            debug_count: 0,
            info_count: 0,
            warn_count: 0,
            error_count: 0,
            total_count: 0,
        };

        for (level, count) in counts {
            match level.to_uppercase().as_str() {
                "TRACE" => stats.trace_count = count,
                "DEBUG" => stats.debug_count = count,
                "INFO" => stats.info_count = count,
                "WARN" => stats.warn_count = count,
                "ERROR" => stats.error_count = count,
                _ => {}
            }
            stats.total_count += count;
        }

        Ok(stats)
    }
}
