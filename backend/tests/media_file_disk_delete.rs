//! `deleteMediaFileFromDisk` over the real API with real files in a temporary library. Deleting
//! cannot be undone, so this pins what it deletes, what it refuses and who may ask for it.

mod common;

use common::{GqlClient, TestApp, playback};
use serde_json::{Value, json};

const DELETE: &str = r#"mutation D($id: String!) {
  deleteMediaFileFromDisk(mediaFileId: $id) { success reason }
}"#;
const EXISTS: &str = "query($id:String!){mediaFile(id:$id){id}}";

async fn library(admin: &mut GqlClient, root: &std::path::Path) -> String {
    std::fs::create_dir_all(root).unwrap();
    let owner = admin.user_id.clone();
    playback::create(
        admin,
        "Library",
        json!({
            "userId": owner, "name": "Films", "path": root.to_string_lossy(), "libraryType": "movies",
            "autoScan": false, "autoOrganize": false, "namingPattern": "",
            "scanIntervalMinutes": 60, "watchForChanges": false, "scanning": false
        }),
    )
    .await
}

async fn file(admin: &mut GqlClient, library: &str, path: &std::path::Path) -> String {
    playback::create(
        admin,
        "MediaFile",
        json!({
            "libraryId": library, "path": path.to_string_lossy(), "size": 4,
            "addedAt": "2026-11-05T00:00:00Z", "isHdr": false
        }),
    )
    .await
}

async fn record_exists(admin: &mut GqlClient, id: &str) -> bool {
    !admin.query(EXISTS, json!({ "id": id })).await["mediaFile"].is_null()
}

fn result(data: &Value) -> &Value {
    &data["deleteMediaFileFromDisk"]
}

#[test]
fn deletes_only_inside_the_library_and_only_for_an_administrator() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let root = app.path("media/films");
        let library_id = library(&mut admin, &root).await;

        // A file in its own folder: the file, its record and the emptied folder all go.
        let folder = root.join("Film (2020)");
        std::fs::create_dir_all(&folder).unwrap();
        let film = folder.join("Film (2020).mkv");
        std::fs::write(&film, b"film").unwrap();
        let id = file(&mut admin, &library_id, &film).await;
        let data = admin.query(DELETE, json!({ "id": id })).await;
        assert_eq!(result(&data)["success"], true, "{data}");
        assert!(!film.exists());
        assert!(!folder.exists());
        assert!(!record_exists(&mut admin, &id).await);

        // A sidecar keeps its folder.
        let kept = root.join("Kept (2021)");
        std::fs::create_dir_all(&kept).unwrap();
        let film = kept.join("Kept (2021).mkv");
        let sidecar = kept.join("Kept (2021).srt");
        std::fs::write(&film, b"film").unwrap();
        std::fs::write(&sidecar, b"text").unwrap();
        let id = file(&mut admin, &library_id, &film).await;
        let data = admin.query(DELETE, json!({ "id": id })).await;
        assert_eq!(result(&data)["success"], true, "{data}");
        assert!(!film.exists());
        assert!(sidecar.exists());

        // A record pointing outside the root is refused, and loses neither file nor record.
        let outside = app.path("media/outside.mkv");
        std::fs::write(&outside, b"keep").unwrap();
        let id = file(&mut admin, &library_id, &outside).await;
        let data = admin.query(DELETE, json!({ "id": id })).await;
        assert_eq!(result(&data)["success"], false, "{data}");
        assert!(result(&data)["reason"].is_string(), "{data}");
        assert!(outside.exists());
        assert!(record_exists(&mut admin, &id).await);

        // A file already gone from disk still clears its record.
        let gone = root.join("Gone.mkv");
        let id = file(&mut admin, &library_id, &gone).await;
        let data = admin.query(DELETE, json!({ "id": id })).await;
        assert_eq!(result(&data)["success"], true, "{data}");
        assert!(!record_exists(&mut admin, &id).await);

        // Signed out, nothing happens.
        let protected = root.join("Protected.mkv");
        std::fs::write(&protected, b"keep").unwrap();
        let id = file(&mut admin, &library_id, &protected).await;
        let response = app.client().post(DELETE, json!({ "id": id })).await;
        assert!(!response.errors().is_empty());
        assert!(protected.exists());
        assert!(record_exists(&mut admin, &id).await);

        app.shutdown().await;
    });
}
