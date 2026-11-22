//! Until the Distance settings tab, `TESSERAE_QUALITY` asks the Server for a Quality curve each
//! time a World opens: `balanced`, `detailed`, or `<unit_m>,<reach_m>`. A preset reaches as far
//! as the Server allows.

use bevy::prelude::*;

use crate::server::quality::QualityCurve;
use crate::server::streaming::StreamingSettings;
use crate::session::SessionApp;

/// The Server clamps it to the Game's ceiling.
const PRESET_REACH_M: f32 = 8192.0;

pub(super) fn plugin(app: &mut App) {
    let Ok(var) = std::env::var("TESSERAE_QUALITY") else {
        return;
    };
    match parse(&var) {
        Ok(curve) => {
            app.insert_resource(Requested(curve))
                .add_session_systems(Update, request);
        }
        Err(e) => warn!("TESSERAE_QUALITY ignored: {e}"),
    }
}

#[derive(Resource)]
struct Requested(QualityCurve);

/// The Server inserts the settings as a World opens, which is when a request is due.
fn request(
    streaming: Option<Res<StreamingSettings>>,
    requested: Res<Requested>,
    mut curves: MessageWriter<QualityCurve>,
) {
    if streaming.is_some_and(|s| s.is_added()) {
        curves.write(requested.0);
    }
}

fn parse(var: &str) -> Result<QualityCurve, String> {
    match var {
        "balanced" => Ok(QualityCurve::balanced(PRESET_REACH_M)),
        "detailed" => Ok(QualityCurve::detailed(PRESET_REACH_M)),
        custom => {
            let number = |s: &str| s.trim().parse::<f32>().map_err(|e| format!("{s:?}: {e}"));
            let (unit, reach) = custom
                .split_once(',')
                .ok_or_else(|| format!("{custom:?} is no preset and no `unit,reach`"))?;
            Ok(QualityCurve {
                unit_m: number(unit)?,
                reach_m: number(reach)?,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_names_a_preset_or_a_unit_and_a_reach() {
        assert_eq!(parse("balanced"), Ok(QualityCurve::balanced(8192.0)));
        assert_eq!(parse("detailed"), Ok(QualityCurve::detailed(8192.0)));
        assert_eq!(
            parse("256, 6000"),
            Ok(QualityCurve {
                unit_m: 256.0,
                reach_m: 6000.0
            })
        );
        assert!(parse("fast").is_err());
        assert!(parse("256,far").is_err());
    }

    #[test]
    fn a_world_opening_sends_the_request_once() {
        let mut app = App::new();
        app.add_message::<QualityCurve>()
            .insert_resource(Requested(QualityCurve::detailed(8192.0)))
            .add_systems(Update, request);
        app.update();
        assert_eq!(sent(&mut app), 0, "no World is open");

        app.init_resource::<StreamingSettings>();
        app.update();
        app.update();
        assert_eq!(sent(&mut app), 1);
    }

    fn sent(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<Messages<QualityCurve>>()
            .drain()
            .count()
    }
}
