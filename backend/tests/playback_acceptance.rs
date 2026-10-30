//! Real media/API fixture and optional long-lived browser acceptance backend.
//! Never copies the live database, reads production configuration or starts workers.
mod common;

use common::{ACCESS_COOKIE, TestApp, TestAppOptions};
use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn real_media_fixture_and_stream_contract() {
    common::run_app_test(|| async {
        // Only this acceptance entrypoint consumes the LAN opt-in. Ordinary test
        // apps remain loopback-only even if the variable exists in a parent shell.
        let bind_ip = std::env::var_os("LIBRARIAN_PLAYBACK_LAN_IP").map(|value| {
            assert!(
                std::env::var_os("LIBRARIAN_PLAYBACK_ACCEPTANCE_DIR").is_some(),
                "LAN acceptance requires a disposable manual-serve directory"
            );
            value
                .to_str()
                .expect("LIBRARIAN_PLAYBACK_LAN_IP must be UTF-8")
                .parse::<std::net::Ipv4Addr>()
                .expect("LIBRARIAN_PLAYBACK_LAN_IP must be an RFC1918 local IPv4 literal")
        });
        let mut extra_cors_origins = vec![
            "http://127.0.0.1:3003".into(),
            "http://localhost:3003".into(),
        ];
        if let Some(ip) = bind_ip {
            extra_cors_origins.push(format!("http://{ip}:3003"));
        }
        let app = TestApp::start_with(TestAppOptions {
            bind_ip,
            with_cast: bind_ip.is_some(),
            with_transcode: true,
            transcode_ffmpeg_path: std::env::var("LIBRARIAN_ACCEPTANCE_FFMPEG").ok(),
            extra_cors_origins,
            ..Default::default()
        })
        .await;
        let password = if bind_ip.is_some() {
            format!("Playback-{}!", uuid::Uuid::new_v4().simple())
        } else {
            common::DEFAULT_PASSWORD.to_string()
        };
        let mut admin = app.client();
        admin
            .register_admin_with_password("admin", "admin@example.test", &password)
            .await;
        if bind_ip.is_some() {
            // Match the visible persisted setting to the disabled runtime setting.
            // Generated entity operations only; never start discovery or LOAD here.
            let settings = admin
                .query(
                    "query { castSettings { edges { node { id } } } }",
                    json!({}),
                )
                .await;
            let rows = settings["castSettings"]["edges"].as_array().unwrap();
            assert!(
                !rows.is_empty(),
                "fixture must have bootstrapped Cast settings"
            );
            for row in rows {
                let updated = admin
                .query(
                    "mutation($id: String!, $input: UpdateCastSettingInput!) { updateCastSetting(id: $id, input: $input) { success error } }",
                    json!({"id": row["node"]["id"], "input": {
                        "autoDiscoveryEnabled": false, "discoveryIntervalSeconds": 3600
                    }}),
                )
                .await;
                assert_eq!(updated["updateCastSetting"]["success"], true, "{updated}");
            }
        }
        let mut manifest = common::playback::seed(&app, &mut admin).await;
        let http = reqwest::Client::new();
        let cookie = format!("{ACCESS_COOKIE}={}", admin.cookies[ACCESS_COOKIE]);
        let id = manifest["movies"]["direct"]["mediaFileId"]
            .as_str()
            .unwrap();
        let stream = format!("{}/api/media/{id}/stream", app.base_url);
        assert_eq!(
            http.get(&stream).send().await.unwrap().status().as_u16(),
            401
        );
        let info = http
            .get(format!("{}/api/media/{id}/info", app.base_url))
            .header("Cookie", &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(info.status().as_u16(), 200);
        let info: serde_json::Value = info.json().await.unwrap();
        assert_eq!(info["exists"], true);
        assert_eq!(info["needs_hls"], false);
        let partial = http
            .get(&stream)
            .header("Cookie", &cookie)
            .header("Range", "bytes=0-31")
            .send()
            .await
            .unwrap();
        assert_eq!(partial.status().as_u16(), 206);
        assert_eq!(partial.bytes().await.unwrap().len(), 32);
        for kind in ["remux", "transcode"] {
            let id = manifest["movies"][kind]["mediaFileId"].as_str().unwrap();
            let url = format!("{}/api/media/{id}/hls/playlist.m3u8", app.base_url);
            let response = http
                .get(&url)
                .header("Cookie", &cookie)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), 200, "{kind} HLS");
            let body = response.text().await.unwrap();
            assert!(body.starts_with("#EXTM3U"), "{kind}: {body}");
            let segment = body
                .lines()
                .find(|line| !line.starts_with('#') && !line.trim().is_empty())
                .unwrap();
            let segment_url = reqwest::Url::parse(&url).unwrap().join(segment).unwrap();
            let response = http
                .get(segment_url)
                .header("Cookie", &cookie)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), 200);
            assert!(!response.bytes().await.unwrap().is_empty());
        }
        manifest["credentials"] = json!({"email":"admin@example.test","password":password});
        if let Ok(directory) = std::env::var("LIBRARIAN_PLAYBACK_ACCEPTANCE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            assert!(
                directory.is_absolute() && directory.is_dir() && !directory.is_symlink(),
                "Use a freshly reserved absolute acceptance directory"
            );
            let ready = directory.join("ready.json");
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&ready)
                .expect("Acceptance targets must not be reused");
            serde_json::to_writer_pretty(file, &manifest).unwrap();
            println!("Real-media acceptance fixture ready: {}", ready.display());
            let started = Instant::now();
            while !directory.join("stop").exists()
                && started.elapsed() < Duration::from_secs(2 * 60 * 60)
            {
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
        app.shutdown().await;
    });
}
