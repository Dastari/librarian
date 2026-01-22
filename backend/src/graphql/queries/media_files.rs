use super::prelude::*;

#[derive(Default)]
pub struct MediaFileQueries;

#[Object]
impl MediaFileQueries {
    /// Get unmatched files for a library (files not linked to any episode)
    async fn unmatched_files(
        &self,
        ctx: &Context<'_>,
        library_id: String,
    ) -> Result<Vec<MediaFile>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .media_files()
            .list_unmatched_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(MediaFile::from_record).collect())
    }

    /// Get count of unmatched files for a library
    async fn unmatched_files_count(&self, ctx: &Context<'_>, library_id: String) -> Result<i32> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let count = db
            .media_files()
            .count_unmatched_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count as i32)
    }

    /// Get all subtitles for a media file
    async fn subtitles_for_media_file(
        &self,
        ctx: &Context<'_>,
        media_file_id: String,
    ) -> Result<Vec<Subtitle>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let file_id = Uuid::parse_str(&media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;

        let records = db
            .subtitles()
            .list_by_media_file(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Subtitle::from_record).collect())
    }

    /// Get all subtitles for an episode (via linked media file)
    async fn subtitles_for_episode(
        &self,
        ctx: &Context<'_>,
        episode_id: String,
    ) -> Result<Vec<Subtitle>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let ep_id = Uuid::parse_str(&episode_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid episode ID: {}", e)))?;

        let records = db
            .subtitles()
            .list_by_episode(ep_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Subtitle::from_record).collect())
    }

    /// Get detailed media file information including all streams
    async fn media_file_details(
        &self,
        ctx: &Context<'_>,
        media_file_id: String,
    ) -> Result<Option<MediaFileDetails>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let file_id = Uuid::parse_str(&media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;

        let file = match db.media_files().get_by_id(file_id).await {
            Ok(Some(f)) => f,
            Ok(None) => return Ok(None),
            Err(e) => return Err(async_graphql::Error::new(e.to_string())),
        };

        let video_streams = db
            .streams()
            .list_video_streams(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let audio_streams = db
            .streams()
            .list_audio_streams(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let subtitles = db
            .subtitles()
            .list_by_media_file(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let chapters = db
            .streams()
            .list_chapters(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Build embedded metadata from the file record
        let embedded_metadata = if file.metadata_extracted_at.is_some() {
            Some(EmbeddedMetadataInfo {
                artist: file.meta_artist.clone(),
                album: file.meta_album.clone(),
                title: file.meta_title.clone(),
                track_number: file.meta_track_number,
                disc_number: file.meta_disc_number,
                year: file.meta_year,
                genre: file.meta_genre.clone(),
                show_name: file.meta_show_name.clone(),
                season: file.meta_season,
                episode: file.meta_episode,
                extracted: true,
                cover_art_base64: file.cover_art_base64.clone(),
                cover_art_mime: file.cover_art_mime.clone(),
                lyrics: file.lyrics.clone(),
            })
        } else {
            // Metadata not yet extracted - return empty with extracted=false
            Some(EmbeddedMetadataInfo {
                extracted: false,
                ..Default::default()
            })
        };

        Ok(Some(MediaFileDetails {
            file: MediaFile::from_record(file),
            video_streams: video_streams
                .into_iter()
                .map(VideoStreamInfo::from_record)
                .collect(),
            audio_streams: audio_streams
                .into_iter()
                .map(AudioStreamInfo::from_record)
                .collect(),
            subtitles: subtitles.into_iter().map(Subtitle::from_record).collect(),
            chapters: chapters.into_iter().map(ChapterInfo::from_record).collect(),
            embedded_metadata,
        }))
    }

    /// Get media file by path
    ///
    /// Returns the media file record if found, null otherwise.
    /// Useful for file browsers to check if a file has been analyzed.
    async fn media_file_by_path(
        &self,
        ctx: &Context<'_>,
        path: String,
    ) -> Result<Option<MediaFile>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let file = db
            .media_files()
            .get_by_path(&path)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(file.map(MediaFile::from_record))
    }

    /// Get media file for a movie
    ///
    /// Returns the media file associated with a movie, if one exists.
    async fn movie_media_file(
        &self,
        ctx: &Context<'_>,
        movie_id: String,
    ) -> Result<Option<MediaFile>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let movie_uuid = Uuid::parse_str(&movie_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        let file = db
            .media_files()
            .get_by_movie_id(movie_uuid)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(file.map(MediaFile::from_record))
    }

    /// Get subtitle settings for a library
    async fn library_subtitle_settings(
        &self,
        ctx: &Context<'_>,
        library_id: String,
    ) -> Result<SubtitleSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let library = db
            .libraries()
            .get_by_id(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

        Ok(SubtitleSettings {
            auto_download: library.auto_download_subtitles.unwrap_or(false),
            languages: library.preferred_subtitle_languages.unwrap_or_default(),
        })
    }
}
