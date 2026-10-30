//! Disposable real-media fixtures. All catalogue writes use generated GraphQL CRUD.
use super::{GqlClient, TestApp};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

fn ffmpeg(args: &[&str]) {
    let binary = std::env::var("LIBRARIAN_ACCEPTANCE_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let output = Command::new(binary)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-threads",
            "1",
            "-filter_threads",
            "1",
        ])
        .args(args)
        .output()
        .expect("FFmpeg must be installed for real-media acceptance");
    assert!(
        output.status.success(),
        "fixture FFmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn generate_media(root: &Path) {
    std::fs::create_dir_all(root).unwrap();
    let subtitle = root.join("captions.srt");
    std::fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:14,000\nAcceptance caption one\n\n2\n00:00:14,000 --> 00:00:30,000\nAcceptance caption two\n").unwrap();
    let video = root.join("direct.mp4");
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=320x180:rate=12",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:sample_rate=48000",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=880:sample_rate=48000",
        "-i",
        subtitle.to_str().unwrap(),
        "-t",
        "30",
        "-map",
        "0:v:0",
        "-map",
        "1:a:0",
        "-map",
        "2:a:0",
        "-map",
        "3:s:0",
        "-c:v",
        "libx264",
        "-preset",
        "ultrafast",
        "-threads",
        "1",
        "-pix_fmt",
        "yuv420p",
        "-g",
        "24",
        "-c:a",
        "aac",
        "-b:a",
        "64k",
        "-c:s",
        "mov_text",
        "-metadata:s:a:0",
        "language=eng",
        "-metadata:s:a:0",
        "title=English tone",
        "-metadata:s:a:1",
        "language=fra",
        "-metadata:s:a:1",
        "title=French tone",
        "-metadata:s:s:0",
        "language=eng",
        "-metadata:s:s:0",
        "title=English captions",
        "-movflags",
        "+faststart",
        video.to_str().unwrap(),
    ]);
    ffmpeg(&[
        "-i",
        video.to_str().unwrap(),
        "-map",
        "0:v:0",
        "-map",
        "0:a:0",
        "-c",
        "copy",
        "-sn",
        root.join("remux.mkv").to_str().unwrap(),
    ]);
    ffmpeg(&[
        "-i",
        video.to_str().unwrap(),
        "-map",
        "0:v:0",
        "-map",
        "0:a:0",
        "-c:v",
        "mpeg4",
        "-q:v",
        "4",
        "-threads",
        "1",
        "-c:a",
        "aac",
        "-sn",
        root.join("transcode.mkv").to_str().unwrap(),
    ]);
    for (name, frequency) in [
        ("track-one.mp3", "440"),
        ("track-two.mp3", "880"),
        ("chapter-one.mp3", "550"),
        ("chapter-two.mp3", "990"),
    ] {
        ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency={frequency}:duration=12"),
            "-c:a",
            "libmp3lame",
            "-b:a",
            "96k",
            root.join(name).to_str().unwrap(),
        ]);
    }
}

pub async fn create(client: &mut GqlClient, kind: &str, input: Value) -> String {
    let field = format!("create{kind}");
    let node = format!("{}{}", kind[..1].to_ascii_lowercase(), &kind[1..]);
    let data = client.query(&format!("mutation($input:Create{kind}Input!){{{field}(input:$input){{success error {node}{{id}}}}}}"), json!({"input": input})).await;
    assert_eq!(data[&field]["success"], true, "{data}");
    data[&field][&node]["id"].as_str().unwrap().into()
}

async fn update(client: &mut GqlClient, kind: &str, id: &str, input: Value) {
    let field = format!("update{kind}");
    let data = client.query(&format!("mutation($id:String!,$input:Update{kind}Input!){{{field}(id:$id,input:$input){{success error}}}}"), json!({"id":id,"input":input})).await;
    assert_eq!(data[&field]["success"], true, "{data}");
}

async fn library(app: &TestApp, client: &mut GqlClient, name: &str, kind: &str) -> String {
    let path = app.path(&format!("media/{kind}"));
    std::fs::create_dir_all(&path).unwrap();
    let owner = client.user_id.clone();
    create(client, "Library", json!({"userId":owner,"name":name,"path":path,"libraryType":kind,"autoScan":false,"autoOrganize":false,"namingPattern":"{title}","scanIntervalMinutes":60,"watchForChanges":false,"scanning":false})).await
}

async fn movie(client: &mut GqlClient, library: &str, name: &str) -> String {
    let owner = client.user_id.clone();
    create(client,"Movie",json!({"userId":owner,"libraryId":library,"title":name,"year":2026,"genres":[],"castNames":[],"productionCountries":[],"spokenLanguages":[],"monitored":false,"wanted":true,"hasFile":false,"overview":"Synthetic media for playback acceptance."})).await
}

#[allow(clippy::too_many_arguments)]
async fn file(
    app: &TestApp,
    client: &mut GqlClient,
    library: &str,
    folder: &str,
    source: &str,
    kind: &str,
    target: &str,
    duration: i32,
) -> String {
    let path = app.path(&format!("media/{folder}/{source}"));
    std::fs::copy(app.path(&format!("acceptance-assets/{source}")), &path).unwrap();
    let mut input = json!({"libraryId":library,"path":path,"size":std::fs::metadata(&path).unwrap().len(),"isHdr":false,"addedAt":"2026-10-27T00:00:00Z","duration":duration});
    input[kind] = json!(target);
    let id = create(client, "MediaFile", input).await;
    let entity_kind = match kind {
        "movieId" => "Movie",
        "episodeId" => "Episode",
        "trackId" => "Track",
        "chapterId" => "Chapter",
        _ => panic!("unknown fixture target"),
    };
    update(client, entity_kind, target, json!({"mediaFileId":id})).await;
    let analyzed = client.query("mutation($id:String!,$path:String!){analyzeMediaFile(mediaFileId:$id,path:$path){success queued message}}",json!({"id":id,"path":path})).await;
    assert_eq!(analyzed["analyzeMediaFile"]["success"], true, "{analyzed}");
    client
        .wait_until(
            std::time::Duration::from_secs(30),
            "real ffprobe analysis",
            "query($id:String!){mediaFile(id:$id){id analyzedAt}}",
            json!({"id":id}),
            |data| data["mediaFile"]["analyzedAt"].is_string(),
        )
        .await;
    id
}

/// Seed all supported media types using actual ffprobe-derived stream metadata.
pub async fn seed(app: &TestApp, client: &mut GqlClient) -> Value {
    let owner = client.user_id.clone().expect("fixture administrator");
    generate_media(&app.path("acceptance-assets"));
    let movies = library(app, client, "Acceptance Movies", "movies").await;
    let shows = library(app, client, "Acceptance TV", "tv").await;
    let music = library(app, client, "Acceptance Music", "music").await;
    let books = library(app, client, "Acceptance Books", "audiobooks").await;
    let mut video = serde_json::Map::new();
    for (key, name, source) in [
        ("direct", "Acceptance Video", "direct.mp4"),
        ("remux", "Acceptance Remux", "remux.mkv"),
        ("transcode", "Acceptance Transcode", "transcode.mkv"),
    ] {
        let id = movie(client, &movies, name).await;
        let file_id = file(app, client, &movies, "movies", source, "movieId", &id, 30).await;
        video.insert(
            key.into(),
            json!({"id":id,"mediaFileId":file_id,"title":name}),
        );
    }
    let missing = movie(client, &movies, "Acceptance Missing File").await;
    let missing_file = create(client,"MediaFile",json!({"libraryId":movies,"movieId":missing,"path":app.path("media/movies/missing.mp4"),"size":1,"isHdr":false,"addedAt":"2026-10-27T00:00:00Z","container":"mp4","videoCodec":"h264","audioCodec":"aac","duration":30})).await;
    update(
        client,
        "Movie",
        &missing,
        json!({"mediaFileId":missing_file}),
    )
    .await;
    video.insert(
        "missing".into(),
        json!({"id":missing,"mediaFileId":missing_file}),
    );
    let show = create(client,"Show",json!({"libraryId":shows,"userId":owner,"name":"Acceptance Show","genres":[],"autoDownload":false,"autoDownloadMode":"NONE"})).await;
    let mut episodes = Vec::new();
    for n in 1..=2 {
        let id = create(client,"Episode",json!({"showId":show,"season":1,"episode":n,"title":format!("Acceptance Episode {n}"),"wanted":true})).await;
        let name = format!("episode-{n}.mp4");
        std::fs::copy(
            app.path("acceptance-assets/direct.mp4"),
            app.path(&format!("acceptance-assets/{name}")),
        )
        .unwrap();
        let media_file = file(app, client, &shows, "tv", &name, "episodeId", &id, 30).await;
        episodes.push(json!({"id":id,"mediaFileId":media_file}));
    }
    let artist = create(
        client,
        "Artist",
        json!({"libraryId":music,"userId":owner,"name":"Acceptance Artist"}),
    )
    .await;
    let album = create(client,"Album",json!({"libraryId":music,"userId":owner,"artistId":artist,"name":"Acceptance Album","genres":[],"autoDownload":false,"autoDownloadMode":"NONE","hasFiles":false})).await;
    let mut tracks = Vec::new();
    for (n, source) in [(1, "track-one.mp3"), (2, "track-two.mp3")] {
        let id=create(client,"Track",json!({"libraryId":music,"albumId":album,"artistId":artist,"title":format!("Acceptance Track {n}"),"trackNumber":n,"explicit":false,"wanted":true,"durationSecs":12})).await;
        let media_file = file(app, client, &music, "music", source, "trackId", &id, 12).await;
        tracks.push(json!({"id":id,"mediaFileId":media_file}));
    }
    let book=create(client,"Audiobook",json!({"libraryId":books,"userId":owner,"title":"Acceptance Audiobook","narrators":[],"autoDownload":false,"autoDownloadMode":"NONE","hasFiles":false})).await;
    let mut chapters = Vec::new();
    for (n, source) in [(1, "chapter-one.mp3"), (2, "chapter-two.mp3")] {
        let id=create(client,"Chapter",json!({"audiobookId":book,"chapterNumber":n,"title":format!("Acceptance Chapter {n}"),"startTimeSecs":0.0,"wanted":true,"durationSecs":12})).await;
        let media_file = file(
            app,
            client,
            &books,
            "audiobooks",
            source,
            "chapterId",
            &id,
            12,
        )
        .await;
        chapters.push(json!({"id":id,"mediaFileId":media_file}));
    }
    json!({"version":1,"backendUrl":app.base_url,"ownerId":owner,"fixtureRoot":app.root,"movies":video,"show":{"id":show,"episodes":episodes},"album":{"id":album,"tracks":tracks},"book":{"id":book,"chapters":chapters},"libraries":{"movies":movies,"shows":shows,"music":music,"books":books}})
}
