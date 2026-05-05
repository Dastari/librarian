use graphql_orm::graphql::loaders::BatchLoadEntity;
use graphql_orm::graphql::orm::DatabaseEntity;
use graphql_orm::{DbRow, sqlx::Row};

macro_rules! impl_string_pk_batch_load {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl BatchLoadEntity for $ty {
                fn batch_column() -> &'static str {
                    <$ty as DatabaseEntity>::PRIMARY_KEY
                }

                fn batch_key_from_row(row: &DbRow) -> Result<String, sqlx::Error> {
                    row.try_get::<String, _>(<$ty as DatabaseEntity>::PRIMARY_KEY)
                }
            }
        )+
    };
}

impl_string_pk_batch_load!(
    super::Album,
    super::AppLog,
    super::AppSetting,
    super::Artist,
    super::ArtworkCache,
    super::AudioStream,
    super::Audiobook,
    super::CastDevice,
    super::CastSession,
    super::CastSetting,
    super::Chapter,
    super::Collection,
    super::Episode,
    super::InviteToken,
    super::Library,
    super::MediaChapter,
    super::MediaFile,
    super::MetadataCache,
    super::Movie,
    super::MovieCastCredit,
    super::NamingPattern,
    super::Notification,
    super::PendingFileMatch,
    super::Person,
    super::PlaybackProgress,
    super::PlaybackSession,
    super::RefreshToken,
    super::RssFeed,
    super::RssFeedItem,
    super::ScheduleCache,
    super::ScheduleSyncState,
    super::Show,
    super::Source,
    super::SourcePriorityRule,
    super::Subtitle,
    super::Torrent,
    super::TorrentFile,
    super::Track,
    super::UsenetDownload,
    super::UsenetServer,
    super::User,
    super::VideoStream,
);

impl_string_pk_batch_load!(super::TorznabCategory);
