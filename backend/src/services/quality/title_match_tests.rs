//! Title matching: the Babylon 5 / Babylon Berlin regression and the names that must still pass.

use super::*;

fn work(name: &str) -> WantedTitle {
    WantedTitle::work(name)
}

fn accepts(wanted: &WantedTitle, release: &str) {
    assert!(
        wanted.matches(release),
        "'{release}' should name {wanted:?}, got: {:?}",
        wanted.mismatch(release)
    );
}

fn rejects(wanted: &WantedTitle, release: &str) {
    assert!(
        !wanted.matches(release),
        "'{release}' should not name {wanted:?}"
    );
}

#[test]
fn babylon_berlin_is_not_babylon_5() {
    let wanted = work("Babylon 5");
    rejects(
        &wanted,
        "Babylon.Berlin.S01.1080p.10bit.WEBRip.6CH.x265.HEVC-PSA",
    );
    rejects(&wanted, "Babylon.Berlin.S03E01.720p.WEB.h264");
    rejects(&wanted, "Babylon.S01.1080p.WEB");
    rejects(&wanted, "Babylon.15.S01.1080p");
    rejects(&wanted, "Babylon.5.The.Road.Home.2023.1080p.BluRay");
    let reason = wanted
        .mismatch("Babylon.Berlin.S01.1080p.10bit.WEBRip.6CH.x265.HEVC-PSA")
        .unwrap();
    assert!(reason.contains("babylon berlin"), "{reason}");

    // And the other way round.
    rejects(&work("Babylon Berlin"), "Babylon.5.S01.1080p.BluRay.x264");
    rejects(&work("Babylon"), "Babylon.5.S01.1080p.BluRay.x264");
}

#[test]
fn babylon_5_releases_still_pass() {
    let wanted = work("Babylon 5");
    accepts(&wanted, "Babylon.5.S01.1080p.BluRay.x264-GRP");
    accepts(&wanted, "Babylon 5 S01E01 Midnight on the Firing Line 720p");
    accepts(&wanted, "Babylon 5 - The Complete Series DVDRip");
    accepts(&wanted, "Babylon.5.Season.1.DVDRip.XviD");
    accepts(&wanted, "Babylon 5 (1994) S01 1080p");
    accepts(&wanted, "Babylon5.S02.720p");
}

#[test]
fn short_and_numbered_titles_pass() {
    accepts(&work("Silo"), "Silo.S02E03.1080p.WEB.h264-ETHEL");
    accepts(
        &work("Silo"),
        "Silo S01 2160p ATVP WEB-DL DDP5.1 Atmos DV HDR H.265",
    );
    rejects(&work("Silo"), "Silo.Rising.S01.1080p");
    accepts(
        &work("Fallout"),
        "Fallout.S01.2160p.AMZN.WEB-DL.DDP5.1.H.265",
    );
    accepts(&work("Fallout"), "Fallout.2024.S01E01.1080p.WEB");
    rejects(&work("Fallout"), "Fallout.New.Vegas.Ultimate.Edition");
    accepts(
        &work("Jujutsu Kaisen"),
        "[SubsPlease] Jujutsu Kaisen - 48 (1080p) [6D1A52C3].mkv",
    );
    accepts(
        &work("Jujutsu Kaisen"),
        "Jujutsu.Kaisen.S02E05.1080p.WEB.H264",
    );
    // The film, not the show.
    rejects(
        &work("Jujutsu Kaisen"),
        "Jujutsu Kaisen 0 2021 1080p BluRay",
    );
    accepts(&work("24"), "24.S05E01.720p.HDTV");
    accepts(&work("9-1-1"), "9-1-1.S07E01.1080p.WEB");
    rejects(&work("9-1-1"), "9-1-1.Lone.Star.S04E01.1080p.WEB");
}

#[test]
fn country_codes_are_tolerated_but_never_swapped() {
    let us = work("The Office (US)");
    accepts(&us, "The.Office.US.S01.1080p.WEB");
    accepts(&us, "The Office S01 1080p");
    accepts(&us, "Office.US.S05E01.720p");
    rejects(&us, "The.Office.UK.S01.1080p");
    rejects(&work("The Office (UK)"), "The.Office.US.S01.1080p");
    accepts(&work("The Office (UK)"), "The.Office.GB.S02.DVDRip");
    // A plain name takes a release that adds a country.
    accepts(&work("Shameless"), "Shameless.US.S11E01.1080p");
    accepts(&work("The Office US"), "The.Office.US.S02.720p");
}

#[test]
fn years_in_names_and_after_them() {
    let doctor = work("Doctor Who (2005)");
    accepts(&doctor, "Doctor.Who.2005.S01.1080p.BluRay");
    accepts(&doctor, "Doctor.Who.S01E01.720p");
    rejects(&doctor, "Doctor.Who.1963.S01.DVDRip");
    accepts(&work("Heat"), "Heat.1995.1080p.BluRay.x264-GRP");
    rejects(&work("Heat"), "Heat.Wave.2022.1080p.WEB");
    accepts(&work("1917"), "1917.2019.2160p.UHD.BluRay");
    accepts(
        &work("Blade Runner 2049"),
        "Blade.Runner.2049.2017.1080p.BluRay",
    );
    rejects(
        &work("Blade Runner 2049"),
        "Blade.Runner.1982.Final.Cut.1080p",
    );
    rejects(&work("Blade Runner"), "Blade.Runner.2049.2017.1080p.BluRay");
}

#[test]
fn punctuation_letters_and_accents() {
    let shield = work("Marvel's Agents of S.H.I.E.L.D.");
    accepts(&shield, "Marvels.Agents.of.S.H.I.E.L.D.S01.1080p");
    accepts(&shield, "Marvels Agents of SHIELD S07E01 720p");
    accepts(&work("Shōgun (2024)"), "Shogun.2024.S01.1080p.WEB");
    accepts(
        &work("Spider-Man: No Way Home"),
        "SpiderMan.No.Way.Home.2021.1080p",
    );
    accepts(
        &work("Law & Order: Special Victims Unit"),
        "Law.and.Order.Special.Victims.Unit.S25E01.1080p",
    );
    accepts(
        &work("Silo"),
        "www.UIndex.org    -    Silo S02E01 1080p WEB",
    );
}

#[test]
fn aliases_from_the_target_and_the_release() {
    accepts(
        &work("Money Heist"),
        "La.Casa.de.Papel.aka.Money.Heist.S01.1080p",
    );
    let amelie = WantedTitle::work_with_aliases(
        "Amélie",
        &[
            Some("Le Fabuleux Destin d'Amélie Poulain".to_string()),
            None,
        ],
    );
    accepts(&amelie, "Amelie.2001.1080p.BluRay");
    accepts(&amelie, "Le.Fabuleux.Destin.d.Amelie.Poulain.2001.1080p");
    rejects(&amelie, "Amelie.Poulain.Returns.2030");
}

#[test]
fn music_needs_the_artist_and_the_album_or_track() {
    let album = WantedTitle::Music {
        artist: "Queen".into(),
        album: Some("A Night at the Opera".into()),
        track: None,
    };
    accepts(&album, "Queen - A Night At The Opera (1975) [FLAC 24-96]");
    accepts(&album, "Queen-A_Night_At_The_Opera-Remastered-WEB-2011-GRP");
    rejects(
        &album,
        "Queens of the Stone Age - Songs for the Deaf (2002) [FLAC]",
    );
    rejects(&album, "Queen - News of the World (1977) [FLAC]");
    let track = WantedTitle::Music {
        artist: "Queen".into(),
        album: Some("A Night at the Opera".into()),
        track: Some("Bohemian Rhapsody".into()),
    };
    accepts(&track, "Queen - Bohemian Rhapsody (Single) [MP3 320]");
    accepts(&track, "Queen - A Night At The Opera (1975) [FLAC]");
    rejects(&track, "Panic! at the Disco - Bohemian Rhapsody [MP3]");
}

#[test]
fn books_need_every_word_of_the_title() {
    let book = WantedTitle::Book {
        title: "The Way of Kings".into(),
        author: Some("Brandon Sanderson".into()),
    };
    accepts(
        &book,
        "Brandon Sanderson - The Way of Kings (Narrated by Kate Reading) [M4B]",
    );
    accepts(&book, "The Way of Kings - Brandon Sanderson 2010 MP3");
    rejects(&book, "Brandon Sanderson - Words of Radiance [M4B]");
    rejects(&book, "Brandon Sanderson - Mistborn [M4B]");
}

#[test]
fn the_title_part_stops_at_the_first_marker() {
    assert_eq!(
        title_part("Babylon.Berlin.S01.1080p.10bit.WEBRip.6CH.x265.HEVC-PSA"),
        vec!["babylon", "berlin"]
    );
    assert_eq!(title_part("Heat.1995.1080p"), vec!["heat"]);
    assert_eq!(
        title_part("[SubsPlease] Jujutsu Kaisen - 48 (1080p)"),
        vec!["jujutsu", "kaisen"]
    );
}

#[test]
fn similarity_prefers_the_closer_title_part() {
    let wanted = work("Babylon 5");
    assert!(wanted.similarity("Babylon.5.S01.1080p") > 0.99);
    assert!(
        wanted.similarity("Babylon.5.S01.1080p") > wanted.similarity("Babylon.Berlin.S01.1080p")
    );
}
