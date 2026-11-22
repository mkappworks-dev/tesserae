//! The Distance preference, and the Quality curve it asks the Server for each time a World
//! opens and each time it is applied. In `dev` and `profiling` builds
//! `TESSERAE_QUALITY=low|medium|high|<unit_m>,<reach_m>` asks for its curve instead.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::profile::Preferences;
use crate::server::LevelRings;
use crate::server::quality::{DeclaredStreaming, QualityCurve};
use crate::server::streaming::StreamingSettings;
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// Detail counts Chunks of 1 m Voxels whatever the World's Voxel size, so a Custom curve means
/// the same metres in every World.
const DETAIL_CHUNK_M: f32 = 32.0;
pub(crate) const MAX_DETAIL_CHUNKS: u32 = 12;
pub(crate) const MIN_REACH_KM: u32 = 2;
const MAX_REACH_KM: u32 = 8;
/// Step 6's 86 MB for levels 1 to 5 at 4 km, less packed quads' 70%, over its ~502 Cell
/// columns of seven Chunks, as the mockup estimates it.
const KB_PER_CHUNK: f32 = 52.0 / 7.0;

pub(super) fn plugin(app: &mut App) {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    if let Ok(var) = std::env::var("TESSERAE_QUALITY") {
        match parse(&var) {
            Ok(switch) => {
                app.insert_resource(switch);
            }
            Err(e) => warn!("TESSERAE_QUALITY ignored: {e}"),
        }
    }
    app.add_session_systems(Update, request);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DistancePreset {
    Low,
    #[default]
    Medium,
    High,
    Custom,
}

impl DistancePreset {
    pub(crate) const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Custom];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Custom => "Custom",
        }
    }
}

/// What the Detail and Reach sliders read: whole Chunks of `u`, and whole km of reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Sliders {
    pub detail_chunks: u32,
    pub reach_km: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Distance {
    pub preset: DistancePreset,
    /// Custom's own values, kept while a preset is chosen. Unclamped, so a Custom curve made in
    /// a World with a far ceiling keeps its reach through one with a near one.
    pub custom: Sliders,
}

impl Default for Distance {
    fn default() -> Self {
        Distance {
            preset: DistancePreset::default(),
            custom: Sliders {
                detail_chunks: 10,
                reach_km: 7,
            },
        }
    }
}

impl Distance {
    pub(crate) fn curve(self, game: DeclaredStreaming) -> QualityCurve {
        match self.preset {
            DistancePreset::Low => QualityCurve::low(game),
            DistancePreset::Medium => QualityCurve::MEDIUM,
            DistancePreset::High => QualityCurve::HIGH,
            DistancePreset::Custom => QualityCurve {
                unit_m: self.custom.detail_chunks as f32 * DETAIL_CHUNK_M,
                reach_m: self.custom.reach_km as f32 * 1000.0,
            },
        }
    }

    /// What the Server streams for this preference in a World that declares `game`.
    pub(crate) fn streamed(self, game: DeclaredStreaming) -> StreamingSettings {
        self.curve(game).applied_to(game.ceiling)
    }

    /// The streamed curve as the sliders show it, within their ends in this World.
    pub(crate) fn sliders(self, game: DeclaredStreaming) -> Sliders {
        let streamed = self.streamed(game);
        let (detail, reach) = (detail_range(game), reach_range(game));
        Sliders {
            detail_chunks: ((streamed.lod_unit_m / DETAIL_CHUNK_M).round() as u32)
                .clamp(*detail.start(), *detail.end()),
            reach_km: ((streamed.lod_distance_m / 1000.0).round() as u32)
                .clamp(*reach.start(), *reach.end()),
        }
    }

    /// Moves a slider, which only Custom allows. Landing on a preset's values chooses that
    /// preset and leaves Custom's memory as it was.
    pub(crate) fn moved(&mut self, game: DeclaredStreaming, set: impl FnOnce(&mut Sliders)) {
        if self.preset != DistancePreset::Custom {
            return;
        }
        let mut custom = self.custom;
        set(&mut custom);
        let shown = Distance {
            preset: DistancePreset::Custom,
            custom,
        }
        .sliders(game);
        let preset = DistancePreset::ALL[..3]
            .iter()
            .copied()
            .find(|&preset| Distance { preset, ..*self }.sliders(game) == shown);
        match preset {
            Some(preset) => self.preset = preset,
            None => self.custom = custom,
        }
    }
}

/// From the World's view distance, which level 0 always covers, to 12 Chunks.
pub(crate) fn detail_range(game: DeclaredStreaming) -> std::ops::RangeInclusive<u32> {
    let view = (game.ceiling.view_distance_m / DETAIL_CHUNK_M).ceil() as u32;
    view..=MAX_DETAIL_CHUNKS.max(view)
}

/// From 2 km to the World's ceiling, at most 8 km.
pub(crate) fn reach_range(game: DeclaredStreaming) -> std::ops::RangeInclusive<u32> {
    let ceiling =
        ((game.ceiling.lod_distance_m / 1000.0).floor() as u32).clamp(MIN_REACH_KM, MAX_REACH_KM);
    MIN_REACH_KM..=ceiling
}

/// One row of the Distance table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Level {
    pub level: u32,
    pub voxel_m: f32,
    pub from_m: f32,
    pub to_m: f32,
    /// The Chunks of the level's mesh shell, its hole for the finer level excluded.
    pub chunks: usize,
    pub mb: f32,
}

/// Every level the Server streams for `streaming`, as the Distance table lists them.
pub(crate) fn levels(streaming: StreamingSettings, world: &WorldSettings) -> Vec<Level> {
    let unit_m = streaming.lod_unit_m.max(streaming.view_distance_m);
    let mut from_m = 0.0;
    (0..crate::server::level_count(&streaming))
        .map(|level| {
            let to_m = if level == 0 {
                streaming.view_distance_m
            } else {
                (unit_m * 2f32.powi(level as i32)).min(streaming.lod_distance_m)
            };
            let chunks = LevelRings::new(streaming, world.cell_size(), level)
                .mesh
                .count();
            let row = Level {
                level,
                voxel_m: world.voxel_size * 2f32.powi(level as i32),
                from_m,
                to_m,
                chunks,
                mb: chunks as f32 * KB_PER_CHUNK / 1024.0,
            };
            from_m = to_m;
            row
        })
        .collect()
}

/// A switch that overrides the Distance preference.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
#[cfg_attr(not(any(feature = "dev", feature = "profiling")), allow(dead_code))]
pub(in crate::client) enum Switch {
    Preset(DistancePreset),
    Curve(QualityCurve),
}

/// The Server inserts the settings as a World opens, which is when a request is first due; an
/// applied preference that asks for another curve sends it again.
pub(in crate::client) fn request(
    streaming: Option<Res<StreamingSettings>>,
    declared: Option<Res<DeclaredStreaming>>,
    preferences: Res<Preferences>,
    switch: Option<Res<Switch>>,
    mut sent: Local<Option<QualityCurve>>,
    mut curves: MessageWriter<QualityCurve>,
) {
    let Some(streaming) = streaming else {
        *sent = None;
        return;
    };
    let game = declared.map_or(DeclaredStreaming::own(*streaming), |declared| *declared);
    let curve = match switch.as_deref() {
        Some(Switch::Curve(curve)) => *curve,
        Some(Switch::Preset(preset)) => Distance {
            preset: *preset,
            ..preferences.distance
        }
        .curve(game),
        None => preferences.distance.curve(game),
    };
    if streaming.is_added() || *sent != Some(curve) {
        curves.write(curve);
        *sent = Some(curve);
    }
}

#[cfg(any(feature = "dev", feature = "profiling", test))]
fn parse(var: &str) -> Result<Switch, String> {
    match var {
        "low" => Ok(Switch::Preset(DistancePreset::Low)),
        "medium" => Ok(Switch::Preset(DistancePreset::Medium)),
        "high" => Ok(Switch::Preset(DistancePreset::High)),
        custom => {
            let number = |s: &str| s.trim().parse::<f32>().map_err(|e| format!("{s:?}: {e}"));
            let (unit, reach) = custom
                .split_once(',')
                .ok_or_else(|| format!("{custom:?} is no preset and no `unit,reach`"))?;
            Ok(Switch::Curve(QualityCurve {
                unit_m: number(unit)?,
                reach_m: number(reach)?,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terra() -> DeclaredStreaming {
        DeclaredStreaming {
            ceiling: StreamingSettings {
                view_distance_m: 160.0,
                lod_distance_m: 8192.0,
                ..default()
            },
            low_reach_m: 3000.0,
        }
    }

    /// A Game whose ceiling is its own reach, as Terra's was before step 22.
    fn near() -> DeclaredStreaming {
        DeclaredStreaming::own(StreamingSettings {
            lod_distance_m: 3000.0,
            ..terra().ceiling
        })
    }

    fn preset(preset: DistancePreset) -> Distance {
        Distance {
            preset,
            ..default()
        }
    }

    #[test]
    fn the_switch_names_a_preset_or_a_unit_and_a_reach() {
        assert_eq!(parse("low"), Ok(Switch::Preset(DistancePreset::Low)));
        assert_eq!(parse("medium"), Ok(Switch::Preset(DistancePreset::Medium)));
        assert_eq!(parse("high"), Ok(Switch::Preset(DistancePreset::High)));
        assert_eq!(
            parse("256, 6000"),
            Ok(Switch::Curve(QualityCurve {
                unit_m: 256.0,
                reach_m: 6000.0
            }))
        );
        assert!(parse("balanced").is_err());
        assert!(parse("256,far").is_err());
    }

    #[test]
    fn each_preset_asks_for_its_curve_and_low_for_the_game_s_own() {
        let game = terra();
        assert_eq!(
            preset(DistancePreset::Low).curve(game),
            QualityCurve {
                unit_m: 160.0,
                reach_m: 3000.0
            }
        );
        assert_eq!(
            preset(DistancePreset::Medium).curve(game),
            QualityCurve::MEDIUM
        );
        assert_eq!(preset(DistancePreset::High).curve(game), QualityCurve::HIGH);
        assert_eq!(
            preset(DistancePreset::Custom).curve(game),
            QualityCurve {
                unit_m: 320.0,
                reach_m: 7000.0
            }
        );
        assert_eq!(Distance::default().preset, DistancePreset::Medium);
    }

    #[test]
    fn the_sliders_show_what_the_world_streams_within_its_own_ends() {
        let game = near();
        assert_eq!(detail_range(game), 5..=12);
        assert_eq!(reach_range(game), 2..=3);
        let shown = |p| preset(p).sliders(game);
        let sliders = |detail_chunks, reach_km| Sliders {
            detail_chunks,
            reach_km,
        };
        assert_eq!(shown(DistancePreset::Low), sliders(5, 3));
        assert_eq!(shown(DistancePreset::Medium), sliders(6, 3));
        assert_eq!(shown(DistancePreset::High), sliders(12, 3));
        assert_eq!(shown(DistancePreset::Custom), sliders(10, 3));

        let far = terra();
        assert_eq!(reach_range(far), 2..=8);
        assert_eq!(preset(DistancePreset::Low).sliders(far), sliders(5, 3));
        assert_eq!(preset(DistancePreset::Medium).sliders(far), sliders(6, 6));
        assert_eq!(preset(DistancePreset::High).sliders(far), sliders(12, 8));
        assert_eq!(preset(DistancePreset::Custom).sliders(far), sliders(10, 7));
    }

    #[test]
    fn the_sliders_move_only_under_custom_and_a_preset_s_values_choose_it() {
        let far = terra();
        let mut medium = preset(DistancePreset::Medium);
        medium.moved(far, |s| s.detail_chunks = 11);
        assert_eq!(medium, preset(DistancePreset::Medium), "locked");

        let mut custom = preset(DistancePreset::Custom);
        custom.moved(far, |s| s.detail_chunks = 11);
        assert_eq!(custom.preset, DistancePreset::Custom);
        assert_eq!(custom.custom.detail_chunks, 11);
        custom.moved(far, |s| s.reach_km = 8);
        custom.moved(far, |s| s.detail_chunks = 12);
        assert_eq!(custom.preset, DistancePreset::High);
        assert_eq!(
            custom.custom,
            Sliders {
                detail_chunks: 11,
                reach_km: 8
            },
            "Custom keeps its own values"
        );
    }

    #[test]
    fn the_table_lists_every_level_the_server_streams() {
        let medium = preset(DistancePreset::Medium).streamed(terra());
        let rows = levels(medium, &WorldSettings::default());
        let rings: Vec<_> = rows.iter().map(|l| (l.voxel_m, l.from_m, l.to_m)).collect();
        assert_eq!(
            rings,
            [
                (1.0, 0.0, 160.0),
                (2.0, 160.0, 384.0),
                (4.0, 384.0, 768.0),
                (8.0, 768.0, 1536.0),
                (16.0, 1536.0, 3072.0),
                (32.0, 3072.0, 6000.0),
            ]
        );
        for row in &rows {
            let rings = LevelRings::new(medium, 32.0, row.level);
            assert_eq!(row.chunks, rings.mesh.count(), "level {}", row.level);
        }
        let low = levels(
            preset(DistancePreset::Low).streamed(terra()),
            &WorldSettings::default(),
        );
        assert_eq!(low.len(), 6, "Terra's own doubling");
        assert_eq!(low.last().unwrap().to_m, 3000.0);
    }

    #[test]
    fn a_world_opening_and_an_applied_change_send_the_request() {
        let mut app = App::new();
        app.add_message::<QualityCurve>()
            .init_resource::<Preferences>()
            .add_systems(Update, request);
        app.update();
        assert_eq!(sent(&mut app), vec![], "no World is open");

        app.init_resource::<StreamingSettings>()
            .insert_resource(terra());
        app.update();
        app.update();
        assert_eq!(sent(&mut app), vec![QualityCurve::MEDIUM]);

        app.world_mut()
            .resource_mut::<Preferences>()
            .distance
            .preset = DistancePreset::Low;
        app.update();
        app.world_mut().resource_mut::<Preferences>().compass = true;
        app.update();
        assert_eq!(sent(&mut app), vec![QualityCurve::low(terra())]);

        app.insert_resource(Switch::Preset(DistancePreset::High));
        app.update();
        assert_eq!(sent(&mut app), vec![QualityCurve::HIGH], "the switch wins");
    }

    fn sent(app: &mut App) -> Vec<QualityCurve> {
        app.world_mut()
            .resource_mut::<Messages<QualityCurve>>()
            .drain()
            .collect()
    }
}
