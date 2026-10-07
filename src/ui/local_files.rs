//! The Local files page: audio on this computer, from the folder the
//! listener chose.
//!
//! Every row plays a URI that came from librespot's own lookup, so the row
//! and the file playback resolves are the same entry. Nothing here builds a
//! URI of its own.

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::local_files::LocalFile;
use crate::model::{Action, Page};
use crate::theme::{self, Icon};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(8.0);
    theme::text(
        ui,
        gettext(locale, "Local files"),
        theme::bold(28.0),
        palette.text,
    );

    if !app.settings.local_files.enabled {
        ui.add_space(12.0);
        theme::text(
            ui,
            gettext(
                locale,
                "Local files are off. Turn them on in Settings to play music from this computer.",
            ),
            theme::regular(15.0),
            palette.secondary,
        );
        ui.add_space(12.0);
        if theme::pill_button(ui, &palette, &gettext(locale, "Open settings"), true).clicked() {
            app.actions.push(Action::Open(Page::Settings));
        }
        return;
    }

    ui.add_space(2.0);
    toolbar(app, ui);

    if app.local_files_scanning && app.local_files.is_empty() {
        ui.add_space(24.0);
        theme::spinner(ui, 20.0, palette.secondary);
        ui.add_space(8.0);
        theme::text(
            ui,
            gettext(locale, "Reading the folder"),
            theme::regular(15.0),
            palette.secondary,
        );
        return;
    }

    if app.local_files.is_empty() {
        ui.add_space(24.0);
        theme::text(
            ui,
            gettext(locale, "No music found here."),
            theme::medium(16.0),
            palette.text,
        );
        ui.add_space(6.0);
        theme::text(
            ui,
            gettext(
                locale,
                "Spotifast reads mp3, mp4, m4p, and flac files, and finds each one by the artist, album, and title its tags carry.",
            ),
            theme::regular(14.0),
            palette.secondary,
        );
        return;
    }

    ui.add_space(10.0);
    theme::text(
        ui,
        locale.local_file_count(app.local_files.len() as u32),
        theme::regular(13.0),
        palette.dim,
    );
    ui.add_space(6.0);

    // The whole list is the playing context, so a row plays it from there and
    // Next carries on through the folder rather than stopping at one file.
    let uris: Vec<String> = app
        .local_files
        .iter()
        .map(|file| file.uri.clone())
        .collect();
    let files = app.local_files.clone();
    for (index, file) in files.iter().enumerate() {
        row(app, ui, index, file, &uris);
    }
}

/// The folder being read, and the button that reads it again.
fn toolbar(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    let folder = app.settings.local_files.directory();
    ui.horizontal(|ui| {
        theme::text(
            ui,
            folder.display().to_string(),
            theme::regular(13.0),
            palette.dim,
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if app.local_files_scanning {
                theme::spinner(ui, 16.0, palette.secondary);
            } else if theme::pill_button(ui, &palette, &gettext(locale, "Rescan"), false).clicked()
            {
                app.actions.push(Action::RescanLocalFiles);
            }
            if theme::pill_button(ui, &palette, &gettext(locale, "Settings"), false).clicked() {
                app.actions.push(Action::Open(Page::Settings));
            }
        });
    });
}

/// Draws one file and reports whether it was chosen. A right-click queues it
/// instead, which is the one thing the queue can do with a local file: it can
/// be played and queued, but Spotify's API will not take it into a playlist.
fn row(app: &mut App, ui: &mut egui::Ui, index: usize, file: &LocalFile, uris: &[String]) {
    let palette = app.palette;
    let locale = app.locale;
    let compact = app.settings.tracklist_compact;
    let height = if compact { 34.0 } else { 44.0 };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let playing = app.now_playing().is_some_and(|now| now.uri == file.uri);
    let hovered = response.hovered();

    if ui.is_rect_visible(rect) {
        if hovered {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(theme::RADIUS),
                palette.surface_hover,
            );
        }
        let left = rect.left() + 8.0;
        let marker = Rect::from_center_size(pos2(left + 9.0, rect.center().y), vec2(18.0, 18.0));
        if hovered {
            theme::paint_icon(
                ui,
                Icon::PlayFilled,
                marker,
                16.0,
                if playing {
                    palette.accent
                } else {
                    palette.text
                },
            );
        } else if playing {
            theme::paint_icon(ui, Icon::AudioLines, marker, 16.0, palette.accent);
        } else {
            ui.painter().text(
                marker.center(),
                Align2::CENTER_CENTER,
                format!("{}", index + 1),
                theme::regular(13.0),
                palette.dim,
            );
        }

        let right = rect.right() - 10.0;
        let duration = crate::util::format_duration_ms(file.duration.as_millis() as u32);
        let duration_rect =
            Rect::from_min_max(pos2(right - 52.0, rect.top()), pos2(right, rect.bottom()));
        ui.painter().text(
            pos2(right, rect.center().y),
            Align2::RIGHT_CENTER,
            duration,
            theme::regular(13.0),
            palette.dim,
        );

        let text_left = left + 30.0;
        let text_width = (duration_rect.left() - 12.0 - text_left).max(40.0);
        let title_color = if playing {
            palette.accent
        } else {
            palette.text
        };
        if compact {
            paint_line(
                ui,
                pos2(text_left, rect.center().y),
                text_width,
                &file.title,
                theme::medium(14.0),
                title_color,
            );
        } else {
            paint_line(
                ui,
                pos2(text_left, rect.center().y - 8.0),
                text_width,
                &file.title,
                theme::medium(14.0),
                title_color,
            );
            let artist = if file.artist_missing() {
                gettext(locale, "Unknown artist").to_string()
            } else {
                file.artist.clone()
            };
            let detail = if file.album.is_empty() {
                artist
            } else {
                format!("{artist} \u{2022} {}", file.album)
            };
            paint_line(
                ui,
                pos2(text_left, rect.center().y + 10.0),
                text_width,
                &detail,
                theme::regular(13.0),
                palette.secondary,
            );
        }
    }

    response.context_menu(|ui| {
        if ui.button(gettext(locale, "Play")).clicked() {
            app.actions.push(Action::PlayUris {
                uris: uris.to_vec(),
                index: index as u32,
            });
            ui.close();
        }
        if ui.button(gettext(locale, "Add to queue")).clicked() {
            app.actions.push(Action::QueueMany {
                songs: vec![(file.uri.clone(), file.title.clone())],
            });
            ui.close();
        }
    });

    if response.clicked() {
        app.actions.push(Action::PlayUris {
            uris: uris.to_vec(),
            index: index as u32,
        });
    }
}

/// One line of row text, cut to the width it is given.
fn paint_line(
    ui: &egui::Ui,
    at: egui::Pos2,
    width: f32,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
) {
    let galley = crate::bidi::layout(
        ui.painter(),
        text,
        font,
        color,
        width,
        1,
        Some(crate::bidi::ELLIPSIS),
    );
    let pos = pos2(at.x, at.y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, color);
}

/// A line under the header, so the folder and its count read as one block.
pub fn divider(ui: &egui::Ui, palette: &theme::Palette) {
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0, palette.outline),
    );
}
