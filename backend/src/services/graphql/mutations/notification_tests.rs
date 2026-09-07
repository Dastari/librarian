use super::notifications::NotificationFeedMutations;
use crate::graphql::entities::*;
use crate::services::graphql::auth::AuthUser;
use crate::services::{DatabaseServiceConfig, ServicesManager};
use async_graphql::{EmptySubscription, Object, Request, Schema};

struct TestQuery;
#[Object]
impl TestQuery {
    async fn ping(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn notification_audit_bulk_read_is_owner_scoped_and_keeps_issues_unresolved() {
    let temp = tempfile::tempdir().unwrap();
    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: format!(
                "sqlite://{}",
                temp.path().join("notifications.db").display()
            ),
            connect_timeout: std::time::Duration::from_secs(5),
        })
        .start()
        .await
        .unwrap();
    let database = services.get_database().await.unwrap();
    let db = database.pool();
    for (owner, count) in [("owner-a", 125), ("owner-b", 1)] {
        for _ in 0..count {
            Notification::insert(
                db,
                CreateNotificationInput {
                    user_id: owner.into(),
                    notification_type: "WARNING".into(),
                    category: "ORGANIZATION".into(),
                    title: "Conflict".into(),
                    message: "A target exists".into(),
                    library_id: None,
                    torrent_id: None,
                    media_file_id: None,
                    pending_match_id: None,
                    action_type: None,
                    action_data: None,
                    read_at: None,
                    resolved_at: None,
                    resolution: None,
                },
            )
            .await
            .unwrap();
        }
        LibraryScanIssue::insert(
            db,
            CreateLibraryScanIssueInput {
                user_id: owner.into(),
                scan_run_id: "fixture-run".into(),
                library_id: "fixture-library".into(),
                media_file_id: None,
                stage: "ANALYZE".into(),
                issue_code: "FFPROBE_MALFORMED_OUTPUT".into(),
                severity: "WARNING".into(),
                message: "Analysis failed".into(),
                remediation: None,
                details_json: None,
                occurrence_count: 1,
                read_at: None,
                resolved_at: None,
                resolution: None,
            },
        )
        .await
        .unwrap();
    }
    let schema = Schema::build(TestQuery, NotificationFeedMutations, EmptySubscription)
        .data(db.clone())
        .finish();
    let query = "mutation { markAllNotificationsRead { notificationCount scanIssueCount } }";
    assert!(
        !schema.execute(query).await.errors.is_empty(),
        "anonymous acknowledgement must fail"
    );
    let owner = AuthUser {
        user_id: "owner-a".into(),
        email: None,
        role: Some("member".into()),
    };
    let response = schema
        .execute(Request::new(query).data(owner.clone()))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let result = response.data.into_json().unwrap();
    assert_eq!(result["markAllNotificationsRead"]["notificationCount"], 125);
    assert_eq!(result["markAllNotificationsRead"]["scanIssueCount"], 1);
    let notifications = Notification::query(db.pool()).fetch_all().await.unwrap();
    for notification in notifications {
        assert_eq!(
            notification.read_at.is_some(),
            notification.user_id == "owner-a"
        );
        assert!(notification.resolved_at.is_none());
    }
    let issues = LibraryScanIssue::query(db.pool())
        .fetch_all()
        .await
        .unwrap();
    for issue in issues {
        assert_eq!(issue.read_at.is_some(), issue.user_id == "owner-a");
        assert!(issue.resolved_at.is_none());
    }
    let repeat = schema
        .execute(Request::new(query).data(owner))
        .await
        .data
        .into_json()
        .unwrap();
    assert_eq!(repeat["markAllNotificationsRead"]["notificationCount"], 0);
    assert_eq!(repeat["markAllNotificationsRead"]["scanIssueCount"], 0);
    // Administrators also acknowledge only their own feed, not other members' records.
    let admin = AuthUser::system_admin_for("owner-a");
    let repeat = schema
        .execute(Request::new(query).data(admin))
        .await
        .data
        .into_json()
        .unwrap();
    assert_eq!(repeat["markAllNotificationsRead"]["notificationCount"], 0);
}
