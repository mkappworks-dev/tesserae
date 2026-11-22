//! The Distance page: the Quality curve's preset and sliders, and a table of the levels it
//! streams in this World.

use super::*;
use crate::client::quality::{self, DistancePreset, Level};
use crate::client::theme;
use crate::server::quality::QualityCurve;
use crate::server::streaming::StreamingSettings;
use crate::world::WorldSettings;

/// The distances of the open World, or the engine's while none is.
pub(in crate::client) type WorldDistances<'w> = (
    Option<Res<'w, crate::server::quality::DeclaredStreaming>>,
    Option<Res<'w, WorldSettings>>,
);

pub(super) fn game(distances: &WorldDistances) -> (StreamingSettings, WorldSettings) {
    (
        distances.0.as_ref().map(|d| d.0).unwrap_or_default(),
        distances.1.as_deref().cloned().unwrap_or_default(),
    )
}

fn preset_hint(preset: DistancePreset, game: StreamingSettings) -> String {
    match preset {
        DistancePreset::Low => format!(
            "The World's own rings, to {}: lightest on memory. The default. Choose Custom to set \
             your own.",
            metres(QualityCurve::low(game).reach_m)
        ),
        DistancePreset::Medium => {
            "Holds each Voxel size further, to 6 km. Choose Custom to set your own.".into()
        }
        DistancePreset::High => {
            "Sharpest and furthest, to 8 km: most memory. Choose Custom to set your own.".into()
        }
        DistancePreset::Custom => "Your own reach and detail, kept while you try a preset.".into(),
    }
}

pub(super) fn distance(
    list: &mut ChildSpawnerCommands,
    p: &Preferences,
    (game, world): (StreamingSettings, &WorldSettings),
) {
    let d = p.distance;
    let locked = d.preset != DistancePreset::Custom;
    let shown = d.sliders(game);
    group(list, "Distant terrain", |card| {
        setting_row(card, "Preset", &preset_hint(d.preset, game), |row| {
            segment(row, |buttons| {
                for preset in DistancePreset::ALL {
                    option(
                        buttons,
                        preset.label(),
                        Operation::Distance(preset),
                        d.preset == preset,
                    );
                }
            })
        });
        setting_row(
            card,
            "Reach",
            "How far distant terrain is drawn, in km.",
            |row| {
                let max = *quality::reach_range(game).end();
                locked_slider(row, Scalar::Reach(max), shown.reach_km as f32, locked)
            },
        );
        setting_row(
            card,
            "Detail",
            "How many of its own Chunks each Voxel size reaches. Higher is sharper, and more memory.",
            |row| {
                let min = *quality::detail_range(game).start();
                locked_slider(row, Scalar::Detail(min), shown.detail_chunks as f32, locked)
            },
        );
    });
    let streamed = d.streamed(game);
    table(
        list,
        &quality::levels(streamed, world),
        streamed.lod_distance_m,
    );
    crate::client::menu_ui::paragraph(
        list,
        "An estimate of distant terrain before trees and buildings; the Game sets the most its World allows.",
        theme::TEXT_CAPTION,
        theme::muted(),
    );
    list.spawn(Node {
        height: Val::Px(12.0),
        flex_shrink: 0.0,
        ..default()
    });
    crate::client::menu_ui::paragraph(
        list,
        format!(
            "The nearest {} is always full detail; the Game sets how far it allows.",
            metres(game.view_distance_m)
        ),
        theme::TEXT_SMALL,
        theme::muted(),
    );
}

pub(super) fn metres(m: f32) -> String {
    if m >= 1000.0 {
        format!("{} km", (m / 100.0).round() / 10.0)
    } else {
        format!("{} m", m.round())
    }
}

fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

// The mockup's translucent whites over the sheet, made opaque: Bevy blends alpha in linear
// space, which would draw them several times brighter.
const TABLE_EDGE: Color = Color::srgb_u8(0x23, 0x23, 0x24);
const TABLE_RULE: Color = Color::srgb_u8(0x1a, 0x1a, 0x1b);
const TABLE_HEAD: Color = Color::srgb_u8(0x15, 0x15, 0x16);
const FULL_DETAIL: Color = Color::srgb_u8(0x18, 0x18, 0x18);
const RING_TRACK: Color = Color::srgb_u8(0x1c, 0x1c, 0x1d);
/// What a level's chip and ring fade towards as its Voxels grow.
const COARSE: Color = Color::srgb_u8(0x5d, 0x6a, 0x72);

/// Level, Voxel, Ring, Chunks, Memory; the ring takes what the others leave.
const COLUMNS: [Option<f32>; 5] = [Some(64.0), Some(64.0), None, Some(120.0), Some(90.0)];

fn shade(level: u32) -> Color {
    let fraction = (1.0 - level as f32 * 0.13).max(0.0);
    COARSE.mix(&theme::SAGE, fraction)
}

fn table(list: &mut ChildSpawnerCommands, levels: &[Level], reach_m: f32) {
    let coarse = &levels[1..];
    let total_mb: f32 = coarse.iter().map(|l| l.mb).sum();
    let total_chunks: usize = coarse.iter().map(|l| l.chunks).sum();
    list.spawn((
        Node {
            width: Val::Percent(100.0),
            margin: UiRect::vertical(Val::Px(8.0)),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            overflow: Overflow::clip(),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(TABLE_EDGE),
    ))
    .with_children(|table| {
        row(table, Some(TABLE_HEAD), true, |cells| {
            for (i, name) in ["Level", "Voxel", "Ring", "Chunks", "Memory"]
                .into_iter()
                .enumerate()
            {
                cell(cells, i, |c| {
                    c.spawn((
                        Text::new(name.to_uppercase()),
                        theme::weighted(theme::TEXT_TAG, 600),
                        bevy::text::LetterSpacing::Px(theme::TEXT_TAG * 0.08),
                        TextColor(theme::muted()),
                        Pickable::IGNORE,
                    ));
                });
            }
        });
        for level in levels {
            level_row(table, level, reach_m);
        }
        row(table, Some(TABLE_HEAD), false, |cells| {
            cells
                .spawn(Node {
                    flex_grow: 1.0,
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(9.0)),
                    ..default()
                })
                .with_children(|c| {
                    bold(
                        c,
                        format!("{} coarse levels to {}", coarse.len(), metres(reach_m)),
                    )
                });
            cell(cells, 3, |c| bold(c, thousands(total_chunks)));
            cell(cells, 4, |c| bold(c, format!("{total_mb:.0} MB")));
        });
    });
}

fn level_row(table: &mut ChildSpawnerCommands, level: &Level, reach_m: f32) {
    let colour = shade(level.level);
    let full = level.level == 0;
    row(table, full.then_some(FULL_DETAIL), true, |cells| {
        cell(cells, 0, |c| {
            c.spawn((
                Node {
                    width: Val::Px(22.0),
                    height: Val::Px(22.0),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(colour),
                Pickable::IGNORE,
            ))
            .with_child((
                Text::new(level.level.to_string()),
                theme::weighted(theme::TEXT_CAPTION, 600),
                TextColor(theme::SAGE_INK),
                Pickable::IGNORE,
            ));
        });
        cell(cells, 1, |c| plain(c, format!("{} m", level.voxel_m)));
        cell(cells, 2, |c| {
            c.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                ..default()
            })
            .with_children(|ring| {
                plain(
                    ring,
                    format!("{} – {}", metres(level.from_m), metres(level.to_m)),
                );
                let percent = |m: f32| Val::Percent(m / reach_m * 100.0);
                ring.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(6.0),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(RING_TRACK),
                    Pickable::IGNORE,
                ))
                .with_child((
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(level.from_m),
                        width: percent(level.to_m - level.from_m),
                        min_width: Val::Px(3.0),
                        height: Val::Percent(100.0),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(colour),
                    Pickable::IGNORE,
                ));
            });
        });
        if full {
            cell(cells, 3, full_detail);
            cell(cells, 4, |c| plain(c, "—"));
        } else {
            cell(cells, 3, |c| plain(c, thousands(level.chunks)));
            cell(cells, 4, |c| plain(c, format!("{:.1} MB", level.mb)));
        }
    });
}

fn row(
    table: &mut ChildSpawnerCommands,
    fill: Option<Color>,
    ruled: bool,
    cells: impl FnOnce(&mut ChildSpawnerCommands),
) {
    table
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                border: UiRect::bottom(Val::Px(if ruled { theme::RULE } else { 0.0 })),
                ..default()
            },
            BorderColor::all(TABLE_RULE),
            BackgroundColor(fill.unwrap_or(Color::NONE)),
        ))
        .with_children(cells);
}

/// The ring and the first column read from the left, the numbers from the right.
fn cell(
    cells: &mut ChildSpawnerCommands,
    column: usize,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    let left = matches!(column, 0 | 2);
    cells
        .spawn(Node {
            width: COLUMNS[column].map_or(Val::Auto, Val::Px),
            flex_grow: if COLUMNS[column].is_none() { 1.0 } else { 0.0 },
            flex_shrink: 0.0,
            padding: UiRect::axes(Val::Px(12.0), Val::Px(9.0)),
            justify_content: if left {
                JustifyContent::FlexStart
            } else {
                JustifyContent::FlexEnd
            },
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(content);
}

fn full_detail(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            },
            BackgroundColor(theme::PILL),
            BorderColor::all(theme::PILL_EDGE),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new("FULL DETAIL"),
            theme::font(theme::TEXT_EYEBROW),
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            TextColor(theme::PILL_INK),
            Pickable::IGNORE,
        ));
}

fn plain(parent: &mut ChildSpawnerCommands, value: impl Into<String>) {
    parent.spawn((
        Text::new(value),
        theme::tabular(theme::font(theme::TEXT_CAPTION)),
        TextColor(theme::INK),
        Pickable::IGNORE,
    ));
}

fn bold(parent: &mut ChildSpawnerCommands, value: impl Into<String>) {
    parent.spawn((
        Text::new(value),
        theme::tabular(theme::weighted(theme::TEXT_CAPTION, 600)),
        TextColor(theme::INK),
        Pickable::IGNORE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_read_as_the_table_writes_them() {
        assert_eq!(metres(160.0), "160 m");
        assert_eq!(metres(3000.0), "3 km");
        assert_eq!(metres(1536.0), "1.5 km");
        assert_eq!(thousands(7), "7");
        assert_eq!(thousands(1234), "1,234");
        assert_eq!(thousands(123_456), "123,456");
    }
}
