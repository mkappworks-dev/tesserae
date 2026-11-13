//! The prompt under the reticle: what Use would do to the Usable Voxel aimed
//! at, or that it is out of reach, as the Server's aim answer says. Nothing
//! here reads the World (ADR-0002).

use bevy::prelude::*;

use super::actions::{Action, Bindings, Labels};
use super::aim::{Sent, Target};
use super::edit::{Catalogue, Holding, Hotbar};
use super::theme;
use crate::server::tool::UseRefusal;
use crate::session::SessionApp;

/// Below the reticle's centre, in pixels, to the prompt's centre.
const BELOW_RETICLE_PX: f32 = 37.0;
const PROMPT_BACKGROUND: Color = Color::srgba_u8(0x0b, 0x0b, 0x0d, 0xcc);
const PROMPT_INK: Color = Color::srgb_u8(0xf1, 0xf0, 0xed);
const REFUSED_INK: Color = Color::srgb_u8(0xa9, 0xad, 0xb6);
const KEY_EDGE: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x4d);
const KEY_BACKGROUND: Color = Color::srgba_u8(0, 0, 0, 0xbb);

#[derive(Component)]
struct Prompt;
#[derive(Component)]
struct PromptRow;
#[derive(Component)]
struct PromptKey;
#[derive(Component)]
struct PromptText;

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(Startup, spawn)
        .add_session_systems(Update, show.after(super::aim::receive));
}

/// What the prompt says: the verb after the Use key, or a refusal alone.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Wording {
    Verb(String),
    Refused(&'static str),
}

/// What the Player calls a Usable Voxel of `material`: its declared name.
pub(super) fn material_title(catalogue: &Catalogue, material: tesserae_core::MaterialId) -> String {
    catalogue
        .0
        .material(material)
        .map_or_else(String::new, |m| super::menu_ui::declared_title(&m.name))
}

/// The prompt for the current aim answer; none with a Tool in hand, whose
/// preview is its own, or when Use would eat.
fn wording(
    target: &Target,
    sent: &Sent,
    now: f64,
    (hotbar, catalogue): (&Hotbar, &Catalogue),
) -> Option<Wording> {
    if matches!(hotbar.held(), Holding::Tool(_)) || !target.current(sent, now) {
        return None;
    }
    match target.0.use_aim? {
        Ok(aimed) => aimed.usable.map(|usable| {
            Wording::Verb(usable.verb(&material_title(catalogue, aimed.voxel.material())))
        }),
        Err(UseRefusal::OutOfReach) => Some(Wording::Refused("Out of reach")),
        Err(_) => None,
    }
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Prompt,
            Visibility::Hidden,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                top: Val::Px(BELOW_RETICLE_PX),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                PromptRow,
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    padding: UiRect::new(Val::Px(4.0), Val::Px(8.0), Val::Px(3.0), Val::Px(3.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(PROMPT_BACKGROUND),
            ))
            .with_children(|row| {
                row.spawn((
                    PromptKey,
                    Node {
                        min_width: Val::Px(20.0),
                        justify_content: JustifyContent::Center,
                        padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                        border: UiRect {
                            bottom: Val::Px(2.0),
                            ..UiRect::all(Val::Px(1.0))
                        },
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BorderColor::all(KEY_EDGE),
                    BackgroundColor(KEY_BACKGROUND),
                    Text::new(""),
                    theme::font(10.0),
                    TextColor(PROMPT_INK),
                ));
                row.spawn((
                    PromptText,
                    Text::new(""),
                    theme::font(theme::TEXT_CAPTION),
                    TextColor(PROMPT_INK),
                ));
            });
        });
}

type PromptParts<'w, 's> = (
    Query<'w, 's, &'static mut Visibility, With<Prompt>>,
    Query<'w, 's, &'static mut Node, (With<PromptRow>, Without<PromptKey>)>,
    Query<'w, 's, (&'static mut Text, &'static mut Node), (With<PromptKey>, Without<PromptRow>)>,
    Query<
        'w,
        's,
        (&'static mut Text, &'static mut TextColor),
        (With<PromptText>, Without<PromptKey>),
    >,
);

fn show(
    (target, sent, time, hotbar): (Res<Target>, Res<Sent>, Res<Time>, Res<Hotbar>),
    catalogue: Res<Catalogue>,
    (bindings, labels): (Res<Bindings>, Res<Labels>),
    dead: Option<Res<super::death::Dead>>,
    (mut prompt, mut row, mut key, mut text): PromptParts,
) {
    let wording = wording(
        &target,
        &sent,
        time.elapsed_secs_f64(),
        (&hotbar, &catalogue),
    )
    .filter(|_| super::death::alive(dead));
    for mut visibility in &mut prompt {
        visibility.set_if_neq(if wording.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    let Some(wording) = wording else {
        return;
    };
    let (words, refused) = match &wording {
        Wording::Verb(verb) => (verb.as_str(), false),
        Wording::Refused(why) => (*why, true),
    };
    for mut node in &mut row {
        let left = Val::Px(if refused { 8.0 } else { 4.0 });
        if node.padding.left != left {
            node.padding.left = left;
        }
    }
    for (mut label, mut node) in &mut key {
        let display = if refused {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
        let bound = bindings.label(Action::Use, &labels);
        if label.0 != bound {
            label.0 = bound;
        }
    }
    for (mut shown, mut colour) in &mut text {
        if shown.0 != words {
            shown.0 = words.to_owned();
        }
        colour.set_if_neq(TextColor(if refused { REFUSED_INK } else { PROMPT_INK }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::tool::{ToolId, Usable, UseAim, UseTarget};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(super::super::actions::test_plugin)
            .init_resource::<Hotbar>()
            .init_resource::<Target>()
            .init_resource::<Sent>()
            .init_resource::<Catalogue>()
            .add_systems(Startup, spawn)
            .add_systems(Update, show);
        app
    }

    fn door(open: bool) -> UseAim {
        Ok(UseTarget {
            at: [5, 8, 5],
            voxel: tesserae_core::Voxel::AIR,
            corner: Vec3::ZERO,
            usable: Some(Usable::Door { open }),
        })
    }

    /// The prompt after a frame on this answer: its key, if shown, and words.
    fn prompt(
        app: &mut App,
        use_aim: Option<UseAim>,
        tool: Option<ToolId>,
    ) -> Option<(Option<String>, String)> {
        let view = (Vec3::ZERO, Vec3::NEG_Z, None, tool);
        let now = app.world().resource::<Time>().elapsed_secs_f64();
        app.world_mut().resource_mut::<Sent>().last = Some(view);
        let mut target = app.world_mut().resource_mut::<Target>();
        target.1 = Some(view);
        target.0.use_aim = use_aim;
        target.2 = now;
        if let Some(tool) = tool {
            app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::Tool(tool));
        }
        app.update();
        let world = app.world_mut();
        let visible = world
            .query_filtered::<&Visibility, With<Prompt>>()
            .single(world)
            .unwrap();
        if *visible == Visibility::Hidden {
            return None;
        }
        let (key, node) = world
            .query_filtered::<(&Text, &Node), With<PromptKey>>()
            .single(world)
            .unwrap();
        let key = (node.display != Display::None).then(|| key.0.clone());
        let words = world
            .query_filtered::<&Text, With<PromptText>>()
            .single(world)
            .unwrap();
        Some((key, words.0.clone()))
    }

    #[test]
    fn aimed_at_a_door_the_prompt_names_the_use_key_and_what_it_would_do() {
        let mut app = app();
        assert_eq!(
            prompt(&mut app, Some(door(false)), None),
            Some((Some("G".into()), "Open".into()))
        );
        assert_eq!(
            prompt(&mut app, Some(door(true)), None),
            Some((Some("G".into()), "Close".into()))
        );
    }

    #[test]
    fn aimed_at_a_container_the_prompt_names_it() {
        let mut app = app();
        app.world_mut().resource_mut::<Catalogue>().0.materials =
            vec![crate::content::CatalogueMaterial {
                remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
                id: 7,
                name: "t:chest".into(),
                solid: true,
                fluid: None,
                burns: false,
            }];
        let chest = Ok(UseTarget {
            voxel: tesserae_core::Voxel::cube(7),
            usable: Some(Usable::Container),
            ..door(false).unwrap()
        });
        assert_eq!(
            prompt(&mut app, Some(chest), None),
            Some((Some("G".into()), "Open Chest".into()))
        );
    }

    #[test]
    fn aimed_at_a_station_the_prompt_names_it() {
        let mut app = app();
        app.world_mut().resource_mut::<Catalogue>().0.materials =
            vec![crate::content::CatalogueMaterial {
                remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
                id: 9,
                name: "t:workbench".into(),
                solid: true,
                fluid: None,
                burns: false,
            }];
        let station = Ok(UseTarget {
            voxel: tesserae_core::Voxel::cube(9),
            usable: Some(Usable::Station),
            ..door(false).unwrap()
        });
        assert_eq!(
            prompt(&mut app, Some(station), None),
            Some((Some("G".into()), "Use Workbench".into()))
        );
    }

    #[test]
    fn out_of_reach_the_prompt_says_so_without_the_key() {
        let mut app = app();
        assert_eq!(
            prompt(&mut app, Some(Err(UseRefusal::OutOfReach)), None),
            Some((None, "Out of reach".into()))
        );
    }

    #[test]
    fn no_prompt_when_use_would_eat_or_a_tool_is_held() {
        let mut app = app();
        assert_eq!(prompt(&mut app, None, None), None);
        let measured = Ok(UseTarget {
            usable: None,
            ..door(false).unwrap()
        });
        assert_eq!(prompt(&mut app, Some(measured), Some(ToolId(0))), None);
        assert_eq!(
            prompt(&mut app, Some(Err(UseRefusal::OutOfReach)), Some(ToolId(0))),
            None
        );
    }
}
