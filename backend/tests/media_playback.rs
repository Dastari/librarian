//! Real-media delivery acceptance. Run only in the parent's serialized,
//! resource-capped lane: fixtures and audio assertions invoke actual ffmpeg.
mod common;

use common::{GqlClient, TestApp, TestAppOptions, playback};
use librarian::graphql::entities::{CreateInviteTokenInput, InviteToken};
use librarian::services::transcode::TranscodeKind;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

fn get(http: &Client, app: &TestApp, user: &GqlClient, path: &str) -> RequestBuilder {
    let mut request = http.get(format!("{}{path}", app.base_url));
    if let Some(cookie) = user.cookies.get(common::ACCESS_COOKIE) {
        request = request.header("Cookie", format!("{}={cookie}", common::ACCESS_COOKIE));
    }
    request
}

async fn subtitle(
    admin: &mut GqlClient,
    file_id: &str,
    source: &str,
    path: Option<&Path>,
    codec: &str,
) -> String {
    playback::create(
        admin,
        "Subtitle",
        json!({
            "mediaFileId":file_id, "sourceType":source, "filePath":path,
            "streamIndex":if source == "embedded" { Some(3) } else { None },
            "codec":codec, "isDefault":false, "isForced":false, "isHearingImpaired":false
        }),
    )
    .await
}

/// Decode the delivered MPEG-TS audio, then count positive zero crossings.
/// This distinguishes the fixture's 440 Hz and 880 Hz languages through the
/// actual endpoint/ffmpeg map, rather than asserting command-line arguments.
fn delivered_frequency(path: &Path) -> usize {
    let binary = std::env::var("LIBRARIAN_ACCEPTANCE_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let output = Command::new(binary)
        .args(["-nostdin", "-v", "error", "-threads", "1", "-i"])
        .arg(path)
        .args([
            "-map", "0:a:0", "-t", "1", "-ac", "1", "-ar", "8000", "-f", "s16le", "pipe:1",
        ])
        .output()
        .expect("decode delivered audio");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let samples: Vec<_> = output
        .stdout
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
        .collect();
    assert!(
        samples.len() >= 7800,
        "expected one second of decoded audio"
    );
    samples
        .windows(2)
        .filter(|pair| pair[0] <= 0 && pair[1] > 0)
        .count()
}

async fn segment_bytes(
    http: &Client,
    app: &TestApp,
    owner: &GqlClient,
    file_id: &str,
    playlist: &str,
    index: i64,
) -> Vec<u8> {
    let uri = playlist
        .lines()
        .find(|line| !line.starts_with('#') && !line.is_empty())
        .expect("HLS segment");
    assert!(uri.contains(&format!("audioStream={index}")), "{playlist}");
    let response = get(http, app, owner, &format!("/api/media/{file_id}/hls/{uri}"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.bytes().await.unwrap().to_vec()
}

#[test]
fn real_media_delivery_enforces_scope_and_delivers_subtitles_and_selected_audio() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            with_transcode: true,
            transcode_ffmpeg_path: std::env::var("LIBRARIAN_ACCEPTANCE_FFMPEG").ok(),
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let fixture = playback::seed(&app, &mut admin).await;
        let file_id = fixture["movies"]["direct"]["mediaFileId"].as_str().unwrap();
        let other_file = fixture["movies"]["remux"]["mediaFileId"].as_str().unwrap();
        let http = Client::new();
        let info_url = format!("/api/media/{file_id}/info");
        let direct: Value = get(&http, &app, &admin, &info_url)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(direct["needs_hls"], false);
        assert_eq!(direct["transcode_decision"], "none");
        assert_eq!(
            direct["playback_url"],
            format!("/api/media/{file_id}/stream")
        );

        let streams = admin.query("query($id:String!){audioStreams(where:{mediaFileId:{eq:$id}}){edges{node{id streamIndex codec}}} subtitles(where:{mediaFileId:{eq:$id}}){edges{node{id sourceType}}}}",json!({"id":file_id})).await;
        let audio = streams["audioStreams"]["edges"].as_array().unwrap();
        assert_eq!(audio.len(), 2);
        let indices: Vec<_> = audio
            .iter()
            .map(|edge| edge["node"]["streamIndex"].as_i64().unwrap())
            .collect();
        assert_eq!(
            indices,
            vec![1, 2],
            "fixture must use absolute video/audio stream indices"
        );
        let embedded_id = streams["subtitles"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| edge["node"]["sourceType"] == "embedded")
            .unwrap()["node"]["id"]
            .as_str()
            .unwrap();
        let embedded_url = format!("/api/media/{file_id}/subtitles/{embedded_id}.vtt");
        assert_eq!(
            http.get(format!("{}{embedded_url}", app.base_url))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get(
                &http,
                &app,
                &admin,
                &format!("/api/media/{other_file}/subtitles/{embedded_id}.vtt")
            )
            .send()
            .await
            .unwrap()
            .status(),
            StatusCode::NOT_FOUND
        );
        let embedded = get(&http, &app, &admin, &embedded_url)
            .send()
            .await
            .unwrap();
        assert_eq!(embedded.status(), StatusCode::OK);
        assert_eq!(
            embedded.headers()["content-type"],
            "text/vtt; charset=utf-8"
        );
        let text = embedded.text().await.unwrap();
        assert!(text.starts_with("WEBVTT"));
        assert!(
            text.contains("Acceptance caption one") && text.contains("-->"),
            "{text}"
        );

        let sidecar_path = app.path("media/movies/captions.srt");
        std::fs::copy(app.path("acceptance-assets/captions.srt"), &sidecar_path).unwrap();
        let sidecar = subtitle(
            &mut admin,
            file_id,
            "external",
            Some(&sidecar_path),
            "subrip",
        )
        .await;
        let sidecar_url = format!("/api/media/{file_id}/subtitles/{sidecar}.vtt");
        let response = get(&http, &app, &admin, &sidecar_url).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .text()
                .await
                .unwrap()
                .contains("Acceptance caption two")
        );
        let bitmap = subtitle(&mut admin, file_id, "embedded", None, "hdmv_pgs_subtitle").await;
        let outside = app.path("outside.srt");
        std::fs::write(&outside, "1\n00:00:00,000 --> 00:00:01,000\nPrivate\n").unwrap();
        let unsafe_id = subtitle(&mut admin, file_id, "external", Some(&outside), "subrip").await;
        let missing = subtitle(
            &mut admin,
            file_id,
            "external",
            Some(&app.path("media/movies/absent.srt")),
            "subrip",
        )
        .await;
        for (id, status) in [
            (&bitmap, StatusCode::UNSUPPORTED_MEDIA_TYPE),
            (&unsafe_id, StatusCode::FORBIDDEN),
            (&missing, StatusCode::NOT_FOUND),
        ] {
            assert_eq!(
                get(
                    &http,
                    &app,
                    &admin,
                    &format!("/api/media/{file_id}/subtitles/{id}.vtt")
                )
                .send()
                .await
                .unwrap()
                .status(),
                status
            );
        }
        #[cfg(unix)]
        {
            let link = app.path("media/movies/escape.srt");
            std::os::unix::fs::symlink(&outside, &link).unwrap();
            let id = subtitle(&mut admin, file_id, "external", Some(&link), "subrip").await;
            assert_eq!(
                get(
                    &http,
                    &app,
                    &admin,
                    &format!("/api/media/{file_id}/subtitles/{id}.vtt")
                )
                .send()
                .await
                .unwrap()
                .status(),
                StatusCode::FORBIDDEN
            );
        }
        let huge = app.path("media/movies/huge.srt");
        std::fs::File::create(&huge)
            .unwrap()
            .set_len(8 * 1024 * 1024 + 1)
            .unwrap();
        let id = subtitle(&mut admin, file_id, "external", Some(&huge), "subrip").await;
        assert_eq!(
            get(
                &http,
                &app,
                &admin,
                &format!("/api/media/{file_id}/subtitles/{id}.vtt")
            )
            .send()
            .await
            .unwrap()
            .status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );

        // A non-owner cannot use either a valid subtitle ID or an absolute audio index.
        InviteToken::insert(
            &app.db,
            CreateInviteTokenInput {
                token: "playback-member".into(),
                created_by: admin.user_id.clone().unwrap(),
                library_ids: vec![],
                role: "member".into(),
                access_level: "full".into(),
                expires_at: None,
                max_uses: Some(1),
                use_count: 0,
                apply_restrictions: false,
                restrictions_template: None,
                is_active: true,
            },
        )
        .await
        .unwrap();
        let mut member = app.client();
        let registration=member.query(common::REGISTER_MUTATION,json!({"input":{"email":"playback-member@example.test","name":"Playback Member","password":common::DEFAULT_PASSWORD,"inviteToken":"playback-member"}})).await;
        assert_eq!(registration["register"]["success"], true);
        member.user_id = registration["register"]["user"]["id"]
            .as_str()
            .map(str::to_owned);
        for path in [
            &embedded_url,
            &sidecar_url,
            &format!("{info_url}?audioStream=2"),
            &format!("/api/media/{file_id}/hls/playlist.m3u8?audioStream=2"),
            &format!("/api/media/{file_id}/hls/seg_000.ts?audioStream=2"),
        ] {
            assert_eq!(
                get(&http, &app, &member, path)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::NOT_FOUND,
                "{path}"
            );
        }
        let transferred=admin.query("mutation($id:String!,$owner:String!){updateLibrary(id:$id,input:{userId:$owner}){success error}}",json!({"id":fixture["libraries"]["movies"],"owner":member.user_id})).await;
        assert_eq!(
            transferred["updateLibrary"]["success"], true,
            "{transferred}"
        );
        assert_eq!(
            get(&http, &app, &member, &embedded_url)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );

        for index in &indices {
            let info: Value = get(
                &http,
                &app,
                &member,
                &format!("{info_url}?audioStream={index}"),
            )
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
            assert_eq!(info["needs_hls"], true);
            assert_eq!(info["audio_codec"], "aac");
            assert_eq!(info["transcode_decision"], "remux");
            assert_eq!(
                info["playback_url"],
                format!("/api/media/{file_id}/hls/playlist.m3u8?audioStream={index}")
            );
        }
        for (index, status) in [
            (0, StatusCode::NOT_FOUND),
            (-1, StatusCode::BAD_REQUEST),
            (99, StatusCode::NOT_FOUND),
        ] {
            assert_eq!(
                get(
                    &http,
                    &app,
                    &member,
                    &format!("{info_url}?audioStream={index}")
                )
                .send()
                .await
                .unwrap()
                .status(),
                status
            );
        }
        // Stream 2 exists on the direct file, but not on the remux file.
        // A global index lookup would incorrectly authorize this selection.
        for suffix in ["info", "hls/playlist.m3u8", "hls/seg_000.ts"] {
            assert_eq!(
                get(
                    &http,
                    &app,
                    &member,
                    &format!("/api/media/{other_file}/{suffix}?audioStream=2")
                )
                .send()
                .await
                .unwrap()
                .status(),
                StatusCode::NOT_FOUND
            );
        }
        // Two simultaneous starts of one variant must not replace each other.
        let first_url = format!("/api/media/{file_id}/hls/playlist.m3u8?audioStream=1");
        let (first, again) = tokio::join!(
            get(&http, &app, &member, &first_url).send(),
            get(&http, &app, &member, &first_url).send()
        );
        let first = first.unwrap();
        let again = again.unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(again.status(), StatusCode::OK);
        let first = first.text().await.unwrap();
        let transcode = app.services.get_transcode().await.unwrap();
        assert_eq!(transcode.active_session_count().await, 1);
        let first_bytes = segment_bytes(&http, &app, &member, file_id, &first, 1).await;
        let second = get(
            &http,
            &app,
            &member,
            &format!("/api/media/{file_id}/hls/playlist.m3u8?audioStream=2"),
        )
        .send()
        .await
        .unwrap();
        assert_eq!(second.status(), StatusCode::OK);
        let second = second.text().await.unwrap();
        let second_bytes = segment_bytes(&http, &app, &member, file_id, &second, 2).await;
        assert_eq!(transcode.active_session_count().await, 2);
        let first_after = segment_bytes(&http, &app, &member, file_id, &first, 1).await;
        assert_eq!(
            first_bytes, first_after,
            "selecting a second language must preserve the first output"
        );
        for (name, bytes, expected) in [
            ("english.ts", first_bytes, 440usize),
            ("french.ts", second_bytes, 880),
        ] {
            let path = app.path(name);
            std::fs::write(&path, bytes).unwrap();
            let frequency = delivered_frequency(&path);
            assert!(
                frequency.abs_diff(expected) < 35,
                "{name}: expected {expected} Hz, got {frequency}"
            );
        }
        // A different kind for the same selected audio also owns its output.
        let input = app.path("media/movies/direct.mp4");
        let remux_dir = transcode
            .touch_session_with_audio(file_id, TranscodeKind::Remux, Some(1))
            .await
            .unwrap();
        let transcode_dir = transcode
            .get_or_start_session_with_audio(file_id, &input, TranscodeKind::Transcode, Some(1))
            .await
            .unwrap();
        assert_ne!(remux_dir, transcode_dir);
        assert_eq!(transcode.active_session_count().await, 3);
        assert!(remux_dir.exists());
        segment_bytes(&http, &app, &member, file_id, &first, 1).await;

        let stream_url = format!("/api/media/{file_id}/stream");
        let raw = std::fs::read(&input).unwrap();
        let suffix = get(&http, &app, &member, &stream_url)
            .header("Range", "bytes=-17")
            .send()
            .await
            .unwrap();
        assert_eq!(suffix.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            suffix.headers()["content-range"],
            format!("bytes {}-{}/{}", raw.len() - 17, raw.len() - 1, raw.len())
        );
        assert_eq!(
            suffix.bytes().await.unwrap().as_ref(),
            &raw[raw.len() - 17..]
        );
        for range in [
            "bytes=999999999-",
            "bytes=-0",
            "bytes=4-2",
            "bytes=0-1,3-4",
            "bananas",
        ] {
            let response = get(&http, &app, &member, &stream_url)
                .header("Range", range)
                .send()
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::RANGE_NOT_SATISFIABLE,
                "{range}"
            );
            assert_eq!(
                response.headers()["content-range"],
                format!("bytes */{}", raw.len())
            );
        }
        // The same route handles an empty real file without subtracting from zero.
        let empty_path = app.path("media/movies/empty.mp4");
        std::fs::write(&empty_path, []).unwrap();
        let empty=playback::create(&mut admin,"MediaFile",json!({"libraryId":fixture["libraries"]["movies"],"path":empty_path,"size":0,"isHdr":false,"addedAt":"2026-10-27T00:00:00Z"})).await;
        let empty_url = format!("/api/media/{empty}/stream");
        assert_eq!(
            get(&http, &app, &member, &empty_url)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let response = get(&http, &app, &member, &empty_url)
            .header("Range", "bytes=0-")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(response.headers()["content-range"], "bytes */0");

        // Default remux/transcode decisions still produce real HLS, and a
        // missing source is reported before a process can be started.
        for (key, decision) in [("remux", "remux"), ("transcode", "transcode")] {
            let id = fixture["movies"][key]["mediaFileId"].as_str().unwrap();
            let info: Value = get(&http, &app, &member, &format!("/api/media/{id}/info"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            assert_eq!(info["transcode_decision"], decision);
            let playlist = get(&http, &app, &member, info["playback_url"].as_str().unwrap())
                .send()
                .await
                .unwrap();
            assert_eq!(playlist.status(), StatusCode::OK);
            let playlist = playlist.text().await.unwrap();
            let uri = playlist
                .lines()
                .find(|line| !line.starts_with('#') && !line.is_empty())
                .unwrap();
            let segment = get(&http, &app, &member, &format!("/api/media/{id}/hls/{uri}"))
                .send()
                .await
                .unwrap();
            assert_eq!(segment.status(), StatusCode::OK);
            assert!(!segment.bytes().await.unwrap().is_empty());
        }
        let missing_file = fixture["movies"]["missing"]["mediaFileId"]
            .as_str()
            .unwrap();
        for suffix in ["stream", "hls/playlist.m3u8"] {
            assert_eq!(
                get(
                    &http,
                    &app,
                    &member,
                    &format!("/api/media/{missing_file}/{suffix}")
                )
                .send()
                .await
                .unwrap()
                .status(),
                StatusCode::NOT_FOUND
            );
        }

        // Probe a genuine AAC/AC3 file: selecting its second stream must use
        // that stream's codec rather than the MediaFile summary's first AAC.
        let alternate_path = app.path("media/movies/alternate-codec.mkv");
        let binary =
            std::env::var("LIBRARIAN_ACCEPTANCE_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
        let output = Command::new(binary)
            .args(["-nostdin", "-v", "error", "-y", "-threads", "1", "-i"])
            .arg(&input)
            .args([
                "-map", "0:v:0", "-map", "0:a:0", "-map", "0:a:1", "-c:v", "copy", "-c:a:0",
                "copy", "-c:a:1", "ac3", "-sn",
            ])
            .arg(&alternate_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let alternate=playback::create(&mut admin,"MediaFile",json!({"libraryId":fixture["libraries"]["movies"],"path":alternate_path,"size":std::fs::metadata(&alternate_path).unwrap().len(),"isHdr":false,"addedAt":"2026-10-27T00:00:00Z"})).await;
        let analyzed=admin.query("mutation($id:String!,$path:String!){analyzeMediaFile(mediaFileId:$id,path:$path){success queued message}}",json!({"id":alternate,"path":alternate_path})).await;
        assert_eq!(analyzed["analyzeMediaFile"]["success"], true, "{analyzed}");
        admin
            .wait_until(
                Duration::from_secs(30),
                "alternate audio codec analysis",
                "query($id:String!){mediaFile(id:$id){analyzedAt}}",
                json!({"id":alternate}),
                |data| data["mediaFile"]["analyzedAt"].is_string(),
            )
            .await;
        for (index, codec, decision) in [(1, "aac", "remux"), (2, "ac3", "transcode")] {
            let info: Value = get(
                &http,
                &app,
                &member,
                &format!("/api/media/{alternate}/info?audioStream={index}"),
            )
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
            assert_eq!(info["audio_codec"], codec);
            assert_eq!(info["transcode_decision"], decision);
        }
        let playlist = get(
            &http,
            &app,
            &member,
            &format!("/api/media/{alternate}/hls/playlist.m3u8?audioStream=2"),
        )
        .send()
        .await
        .unwrap();
        assert_eq!(playlist.status(), StatusCode::OK);
        let bytes = segment_bytes(
            &http,
            &app,
            &member,
            &alternate,
            &playlist.text().await.unwrap(),
            2,
        )
        .await;
        let path = app.path("selected-ac3.ts");
        std::fs::write(&path, bytes).unwrap();
        assert!(delivered_frequency(&path).abs_diff(880) < 35);

        // A failed child is evicted and the same variant can start after repair.
        let retry_input = app.path("media/movies/repaired.mp4");
        let failed = transcode
            .get_or_start_session_with_audio(
                "retry-fixture",
                &retry_input,
                TranscodeKind::Remux,
                Some(1),
            )
            .await
            .unwrap();
        let started = Instant::now();
        while transcode
            .touch_session_with_audio("retry-fixture", TranscodeKind::Remux, Some(1))
            .await
            .is_some()
        {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "failed ffmpeg child remained reusable"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        std::fs::copy(&input, &retry_input).unwrap();
        let repaired = transcode
            .get_or_start_session_with_audio(
                "retry-fixture",
                &retry_input,
                TranscodeKind::Remux,
                Some(1),
            )
            .await
            .unwrap();
        assert_ne!(failed, repaired);
        assert!(!failed.exists());
        let started = Instant::now();
        while !repaired.join("playlist.m3u8").exists() {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "repaired input did not produce a playlist"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert_eq!(
            transcode
                .get_or_start_session_with_audio(
                    "retry-fixture",
                    &retry_input,
                    TranscodeKind::Remux,
                    Some(1)
                )
                .await
                .unwrap(),
            repaired
        );
        app.shutdown().await;
    });
}
