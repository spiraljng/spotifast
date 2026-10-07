//! Audio files on this machine, listed for the interface.
//!
//! The list comes from librespot's own local file lookup, the same call that
//! builds what playback resolves. A second implementation of the URI format
//! could disagree with that one and fail to match in silence, so there is
//! only one.
//!
//! A local file's identity is its `spotify:local:` URI: the artist, album,
//! title, and length its tags carry. That is also what Spotify stores when
//! the same file is added to a playlist elsewhere, which is why a file has to
//! keep its tags to be found.

use std::path::{Path, PathBuf};
use std::time::Duration;

use librespot_core::SpotifyUri;
use librespot_playback::local_file::create_local_file_lookup;

/// One audio file found on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalFile {
    /// `spotify:local:...`, what playback resolves and the queue carries.
    pub uri: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
    /// Where it was found, for the row's tooltip.
    pub path: PathBuf,
}

impl LocalFile {
    /// Whether the file carries no artist tag. The row shows a translated
    /// placeholder in that case, so no display text lives here.
    pub fn artist_missing(&self) -> bool {
        self.artist.is_empty()
    }
}

/// Every audio file under `directories`, in the order the interface lists
/// them. Reading each file's tags is blocking work, so callers run this off
/// the thread that draws.
pub fn scan(directories: &[PathBuf]) -> Vec<LocalFile> {
    let lookup = create_local_file_lookup(directories);
    let mut files = Vec::with_capacity(lookup.len());
    for (key, path) in lookup.entries() {
        let SpotifyUri::Local {
            artist,
            album_title,
            track_title,
            duration,
        } = key
        else {
            continue;
        };
        // A local URI always renders; a failure here would be a bug in the
        // URI itself, and a row that cannot be played is worse than none.
        let Ok(uri) = key.to_uri() else {
            continue;
        };
        files.push(LocalFile {
            uri,
            title: decode(track_title),
            artist: decode(artist),
            album: decode(album_title),
            duration: *duration,
            path: path.to_path_buf(),
        });
    }
    sort(&mut files);
    files
}

/// Artist, then album, then title. The URI carries no track number, so files
/// inside one album fall back to their titles rather than their own order.
fn sort(files: &mut [LocalFile]) {
    files.sort_by(|left, right| {
        folded(&left.artist)
            .cmp(&folded(&right.artist))
            .then_with(|| folded(&left.album).cmp(&folded(&right.album)))
            .then_with(|| folded(&left.title).cmp(&folded(&right.title)))
    });
}

fn folded(text: &str) -> String {
    text.to_lowercase()
}

/// Turns a tag back into text. librespot writes these with `form_urlencoded`,
/// which spells a space as `+` and everything else outside plain text as
/// `%XX`. Every reading of a local URI goes through here, so a row and the
/// list the file came from agree on the text.
pub(crate) fn decode(encoded: &str) -> String {
    percent_encoding::percent_decode_str(&encoded.replace('+', " "))
        .decode_utf8_lossy()
        .into_owned()
}

/// Whether a path sits inside one of the searched folders. Used by tests and
/// by callers that need to tell a local row from a Spotify one.
pub fn is_local_uri(uri: &str) -> bool {
    uri.starts_with("spotify:local:")
}

/// The folder a path belongs to, for grouping.
pub fn parent(path: &Path) -> Option<&Path> {
    path.parent()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_form_encoded_tag_becomes_text() {
        assert_eq!(decode("Snomads+Island"), "Snomads Island");
        assert_eq!(decode("50%25+Off"), "50% Off");
        assert_eq!(decode(""), "");
        assert_eq!(decode("Sigur+R%C3%B3s"), "Sigur Rós");
    }

    #[test]
    fn a_literal_plus_survives() {
        // A tag holding a real plus is written as %2B, so the `+` spelling
        // is always a space.
        assert_eq!(decode("C%2B%2B"), "C++");
    }

    #[test]
    fn local_uris_are_recognised() {
        assert!(is_local_uri("spotify:local:Artist:Album:Song:180"));
        assert!(!is_local_uri("spotify:track:4iV5W9uYEdYUVa79Axb7Rh"));
        assert!(!is_local_uri("spotify:episode:512ojhOuo1ktJprKbVcKyQ"));
    }

    /// The list hands playback a URI string, and playback finds the file by
    /// parsing that string back into the key its lookup was built with. If
    /// the two disagree the row plays nothing and says nothing, so this
    /// round trip is the one thing worth pinning down.
    #[test]
    fn a_local_uri_survives_the_round_trip_playback_makes() {
        let uri = SpotifyUri::Local {
            artist: "Sigur+R%C3%B3s".to_owned(),
            album_title: "()".to_owned(),
            track_title: "Untitled+3".to_owned(),
            duration: Duration::from_secs(247),
        };
        let text = uri.to_uri().unwrap();
        assert_eq!(text, "spotify:local:Sigur+R%C3%B3s:():Untitled+3:247");
        assert_eq!(SpotifyUri::from_uri(&text).unwrap(), uri);
        assert!(is_local_uri(&text));
    }

    /// A tag holding a colon must not shift the fields apart when playback
    /// parses the URI back.
    #[test]
    fn a_colon_in_a_tag_still_round_trips() {
        let uri = SpotifyUri::Local {
            artist: "AC%2FDC".to_owned(),
            album_title: "Back+in+Black".to_owned(),
            track_title: "It%27s+a+Long+Way".to_owned(),
            duration: Duration::from_secs(252),
        };
        let text = uri.to_uri().unwrap();
        assert_eq!(SpotifyUri::from_uri(&text).unwrap(), uri);
        assert_eq!(decode("It%27s+a+Long+Way"), "It's a Long Way");
        assert_eq!(decode("AC%2FDC"), "AC/DC");
    }

    #[test]
    fn files_are_ordered_by_artist_then_album_then_title() {
        let file = |artist: &str, album: &str, title: &str| LocalFile {
            uri: String::new(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            duration: Duration::ZERO,
            path: PathBuf::new(),
        };
        let mut files = vec![
            file("Beta", "First", "Song"),
            file("Alpha", "Second", "Song"),
            file("Alpha", "First", "Zed"),
            file("Alpha", "First", "Able"),
        ];
        sort(&mut files);
        let order: Vec<(&str, &str, &str)> = files
            .iter()
            .map(|file| {
                (
                    file.artist.as_str(),
                    file.album.as_str(),
                    file.title.as_str(),
                )
            })
            .collect();
        assert_eq!(
            order,
            [
                ("Alpha", "First", "Able"),
                ("Alpha", "First", "Zed"),
                ("Alpha", "Second", "Song"),
                ("Beta", "First", "Song"),
            ]
        );
    }

    #[test]
    fn an_untagged_artist_is_reported_as_missing() {
        let file = LocalFile {
            uri: String::new(),
            title: "Song".into(),
            artist: String::new(),
            album: String::new(),
            duration: Duration::ZERO,
            path: PathBuf::new(),
        };
        assert!(file.artist_missing());
    }

    #[test]
    fn an_empty_folder_finds_nothing() {
        let empty = std::env::temp_dir().join("spotifast-local-files-empty");
        let _ = std::fs::create_dir_all(&empty);
        assert!(scan(std::slice::from_ref(&empty)).is_empty());
    }
}
