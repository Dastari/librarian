-- Add fingerprinting, extended metadata, match tracking, and hunt_individual_items to tables
-- This migration adds support for:
-- 1. Audio fingerprinting (Chromaprint for AcoustID identification)
-- 2. ReplayGain/SoundCheck for loudness normalization
-- 3. Extended metadata from various container formats (iTunes, WMA, APEv2, RIFF/INFO, BWF, DSD, MKV, MOV, AVI)
-- 4. Match type tracking (manual vs automatic) to prevent overwriting manual matches
-- 5. hunt_individual_items flag for partial downloads (search for individual items instead of packs)

-- ============================================================================
-- Match Type Tracking (Critical for manual matching)
-- ============================================================================

-- How the file was matched to a library item: 'automatic', 'manual', or NULL (unmatched)
ALTER TABLE media_files ADD COLUMN match_type TEXT CHECK (match_type IN ('automatic', 'manual'));

-- When the match was last confirmed/verified by a user
ALTER TABLE media_files ADD COLUMN match_confirmed_at TEXT;

-- Who performed the manual match (NULL for automatic matches)
ALTER TABLE media_files ADD COLUMN matched_by_user_id TEXT;

-- ============================================================================
-- Audio Fingerprinting Columns
-- ============================================================================

-- Chromaprint fingerprint hash for audio content identification
-- This is the raw fingerprint data that can be used with AcoustID
ALTER TABLE media_files ADD COLUMN audio_fingerprint TEXT;

-- AcoustID recording ID if matched (UUID format from MusicBrainz)
ALTER TABLE media_files ADD COLUMN acoustid_id TEXT;

-- MusicBrainz recording ID if matched via fingerprint
ALTER TABLE media_files ADD COLUMN musicbrainz_recording_id TEXT;

-- When the fingerprint was last generated
ALTER TABLE media_files ADD COLUMN fingerprint_generated_at TEXT;

-- ============================================================================
-- ReplayGain / SoundCheck / Loudness Columns
-- ============================================================================

-- ReplayGain track gain in dB (for single track playback)
ALTER TABLE media_files ADD COLUMN replaygain_track_gain REAL;

-- ReplayGain track peak (linear, 0.0-1.0+)
ALTER TABLE media_files ADD COLUMN replaygain_track_peak REAL;

-- ReplayGain album gain in dB (for album playback)
ALTER TABLE media_files ADD COLUMN replaygain_album_gain REAL;

-- ReplayGain album peak (linear, 0.0-1.0+)
ALTER TABLE media_files ADD COLUMN replaygain_album_peak REAL;

-- EBU R128 integrated loudness in LUFS (TV/streaming standard)
ALTER TABLE media_files ADD COLUMN r128_integrated_loudness REAL;

-- EBU R128 true peak in dBTP
ALTER TABLE media_files ADD COLUMN r128_true_peak REAL;

-- SoundCheck normalization value (iTunes format, stored as-is)
ALTER TABLE media_files ADD COLUMN soundcheck_value TEXT;

-- ============================================================================
-- Extended Audio Metadata Columns
-- ============================================================================

-- Album artist (distinct from track artist, common in iTunes/M4A)
ALTER TABLE media_files ADD COLUMN meta_album_artist TEXT;

-- Composer (important for classical, soundtracks, iTunes tags)
ALTER TABLE media_files ADD COLUMN meta_composer TEXT;

-- Conductor (classical music)
ALTER TABLE media_files ADD COLUMN meta_conductor TEXT;

-- Label/Publisher
ALTER TABLE media_files ADD COLUMN meta_label TEXT;

-- Catalog number (for releases)
ALTER TABLE media_files ADD COLUMN meta_catalog_number TEXT;

-- ISRC (International Standard Recording Code)
ALTER TABLE media_files ADD COLUMN meta_isrc TEXT;

-- BPM (beats per minute, common in electronic music)
ALTER TABLE media_files ADD COLUMN meta_bpm INTEGER;

-- Initial key (musical key, e.g., "Am", "C#m", common in DJ software)
ALTER TABLE media_files ADD COLUMN meta_initial_key TEXT;

-- Compilation flag (is this part of a compilation album?)
ALTER TABLE media_files ADD COLUMN meta_is_compilation INTEGER DEFAULT 0;

-- Gapless playback info (for seamless album playback)
ALTER TABLE media_files ADD COLUMN meta_gapless_playback INTEGER DEFAULT 0;

-- Rating (0-5 or 0-100 scale, stored as 0-100)
ALTER TABLE media_files ADD COLUMN meta_rating INTEGER;

-- Play count (from embedded metadata)
ALTER TABLE media_files ADD COLUMN meta_play_count INTEGER;

-- ============================================================================
-- Professional Audio Metadata (BWF - Broadcast Wave Format)
-- ============================================================================

-- BWF originator (who created the recording)
ALTER TABLE media_files ADD COLUMN bwf_originator TEXT;

-- BWF originator reference (unique ID from originator)
ALTER TABLE media_files ADD COLUMN bwf_originator_reference TEXT;

-- BWF origination date (YYYY-MM-DD)
ALTER TABLE media_files ADD COLUMN bwf_origination_date TEXT;

-- BWF origination time (HH:MM:SS)
ALTER TABLE media_files ADD COLUMN bwf_origination_time TEXT;

-- BWF time reference (sample count since midnight)
ALTER TABLE media_files ADD COLUMN bwf_time_reference INTEGER;

-- BWF UMID (SMPTE UMID for broadcast identification)
ALTER TABLE media_files ADD COLUMN bwf_umid TEXT;

-- BWF coding history (encoding chain history)
ALTER TABLE media_files ADD COLUMN bwf_coding_history TEXT;

-- ============================================================================
-- Video Container Metadata
-- ============================================================================

-- Director (from video containers like MOV, MKV)
ALTER TABLE media_files ADD COLUMN meta_director TEXT;

-- Producer (from video containers)
ALTER TABLE media_files ADD COLUMN meta_producer TEXT;

-- Copyright notice
ALTER TABLE media_files ADD COLUMN meta_copyright TEXT;

-- Encoder/creation tool used
ALTER TABLE media_files ADD COLUMN meta_encoder TEXT;

-- Content creation date (distinct from file date)
ALTER TABLE media_files ADD COLUMN meta_creation_date TEXT;

-- ============================================================================
-- DSD/High-Resolution Audio Metadata
-- ============================================================================

-- DSD sample rate (if applicable: 2.8MHz, 5.6MHz, 11.2MHz, etc.)
ALTER TABLE media_files ADD COLUMN dsd_sample_rate INTEGER;

-- Whether this is a DSD file (DSF, DFF formats)
ALTER TABLE media_files ADD COLUMN is_dsd INTEGER DEFAULT 0;

-- ============================================================================
-- Indexes for common queries
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_media_files_fingerprint ON media_files(audio_fingerprint) WHERE audio_fingerprint IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_media_files_acoustid ON media_files(acoustid_id) WHERE acoustid_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_media_files_musicbrainz ON media_files(musicbrainz_recording_id) WHERE musicbrainz_recording_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_media_files_needs_fingerprint ON media_files(library_id) WHERE audio_fingerprint IS NULL AND content_type = 'track';
CREATE INDEX IF NOT EXISTS idx_media_files_bpm ON media_files(meta_bpm) WHERE meta_bpm IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_media_files_key ON media_files(meta_initial_key) WHERE meta_initial_key IS NOT NULL;

-- ============================================================================
-- Partial Download Tracking (hunt_individual_items flag)
-- ============================================================================
-- When a download completes but only some items are matched (partial download),
-- this flag is set to TRUE. Auto-hunt will then search for individual missing
-- items instead of complete packs (to avoid re-downloading the same partial release).

-- Albums: search for individual tracks instead of complete album releases
ALTER TABLE albums ADD COLUMN hunt_individual_items INTEGER DEFAULT 0;

-- TV Shows: search for individual episodes instead of season packs  
ALTER TABLE tv_shows ADD COLUMN hunt_individual_items INTEGER DEFAULT 0;

-- Audiobooks: search for individual chapters instead of complete audiobook
ALTER TABLE audiobooks ADD COLUMN hunt_individual_items INTEGER DEFAULT 0;
