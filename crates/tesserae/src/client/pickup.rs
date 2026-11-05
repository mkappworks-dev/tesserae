//! What Pickup did, as the Server tells it: the feed at the bottom right and
//! the Inventory-full notice above the hotbar. Drawn from Server messages only
//! (`docs/adr/0002-client-server-from-day-one.md`).

use bevy::prelude::*;
use tesserae_core::inventory::Inventory;

use super::drops::Looks;
use super::menu::Menu;
use super::menu_ui::font;
use super::settings::Settings;
use crate::server::entities::Pickup;
use crate::server::inventory::InventoryChanged;
use crate::session::SessionApp;

const FEED_S: f32 = 3.0;
const FEED_FADE_S: f32 = 0.4;
const FEED_ROWS: usize = 5;
const ROW_ALPHA: f32 = 0xc7 as f32 / 255.0;

#[derive(Clone, PartialEq, Debug)]
struct FeedRow {
    item: String,
    count: u32,
    age_s: f32,
}

/// Newest first.
#[derive(Resource, Default)]
struct Feed(Vec<FeedRow>);

#[derive(Resource, Default)]
struct FullNotice {
    text: Option<String>,
    held: Option<Inventory>,
}

#[derive(Component)]
struct FeedRoot;
#[derive(Component)]
struct FeedRowNode(usize);
#[derive(Component)]
struct NoticeNode;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Feed>()
        .init_session_state::<FullNotice>()
        .add_session_message::<Pickup>()
        .add_session_message::<InventoryChanged>()
        .add_session_systems(
            Update,
            (
                receive_inventory,
                receive_pickups,
                age_feed,
                show_feed,
                fade_feed,
                show_notice,
            )
                .chain(),
        );
}

/// Room was made: a slot now holds less of its Item, or none.
fn freed(before: &Inventory, now: &Inventory) -> bool {
    before.slots().zip(now.slots()).any(|(b, n)| match (b, n) {
        (Some(b), Some(n)) => b.item != n.item || n.count < b.count,
        (Some(_), None) => true,
        (None, _) => false,
    })
}

fn receive_inventory(mut changes: MessageReader<InventoryChanged>, mut notice: ResMut<FullNotice>) {
    for InventoryChanged(now) in changes.read() {
        if notice
            .held
            .as_ref()
            .is_some_and(|before| freed(before, now))
        {
            notice.text = None;
        }
        notice.held = Some(now.clone());
    }
}

fn label<'a>(looks: &'a Looks, item: &'a str) -> &'a str {
    looks
        .0
        .iter()
        .find(|look| look.name == item)
        .map_or(item, |look| look.label())
}

fn receive_pickups(
    mut pickups: MessageReader<Pickup>,
    looks: Res<Looks>,
    mut feed: ResMut<Feed>,
    mut notice: ResMut<FullNotice>,
) {
    for pickup in pickups.read() {
        if pickup.taken > 0 {
            let earlier = feed.0.iter().position(|r| r.item == pickup.item);
            let before = earlier.map_or(0, |at| feed.0.remove(at).count);
            feed.0.insert(
                0,
                FeedRow {
                    item: pickup.item.clone(),
                    count: before + pickup.taken,
                    age_s: 0.0,
                },
            );
            feed.0.truncate(FEED_ROWS);
        }
        if pickup.left > 0 {
            notice.text = Some(format!(
                "Inventory full · {} {} left on the ground",
                pickup.left,
                label(&looks, &pickup.item)
            ));
        }
    }
}

fn age_feed(time: Res<Time>, mut feed: ResMut<Feed>) {
    if feed.0.is_empty() {
        return;
    }
    for row in &mut feed.0 {
        row.age_s += time.delta_secs();
    }
    feed.0.retain(|r| r.age_s < FEED_S);
}

fn in_play(menu: &Menu, settings: Option<&Settings>) -> bool {
    !menu.open && !settings.is_some_and(|s| s.open)
}

/// The Item's declared colour.
fn swatch(looks: &Looks, item: &str) -> Color {
    looks
        .0
        .iter()
        .find(|look| look.name == item)
        .map_or(Color::WHITE, |look| {
            let [r, g, b, a] = look.color;
            Color::srgba_u8(r, g, b, a)
        })
}

fn show_feed(
    mut commands: Commands,
    feed: Res<Feed>,
    looks: Res<Looks>,
    (menu, settings): (Res<Menu>, Option<Res<Settings>>),
    roots: Query<Entity, With<FeedRoot>>,
    mut shown: Local<Option<Vec<(String, u32)>>>,
) {
    let wanted = in_play(&menu, settings.as_deref()).then(|| {
        feed.0
            .iter()
            .map(|r| (r.item.clone(), r.count))
            .collect::<Vec<_>>()
    });
    if *shown == wanted {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some(rows) = wanted.clone() else {
        *shown = wanted;
        return;
    };
    *shown = wanted;
    commands
        .spawn((
            FeedRoot,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                bottom: Val::Px(58.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|root| {
            for (at, (item, count)) in rows.iter().enumerate() {
                root.spawn((
                    FeedRowNode(at),
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        padding: UiRect {
                            left: Val::Px(6.0),
                            right: Val::Px(12.0),
                            top: Val::Px(4.0),
                            bottom: Val::Px(4.0),
                        },
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(6, 7, 8).with_alpha(ROW_ALPHA)),
                    BorderColor::all(Color::BLACK),
                ))
                .with_children(|row| {
                    row.spawn((
                        Node {
                            width: Val::Px(22.0),
                            height: Val::Px(22.0),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(swatch(&looks, item)),
                    ));
                    row.spawn((
                        Text::new(format!("+{count}")),
                        TextFont {
                            weight: bevy::text::FontWeight(700),
                            ..font(13.0)
                        },
                        TextColor(Color::srgb_u8(0xf6, 0xf7, 0xf1)),
                    ));
                    row.spawn((
                        Text::new(label(&looks, item)),
                        font(13.0),
                        TextColor(Color::srgb_u8(0xc9, 0xcc, 0xd1)),
                    ));
                });
            }
        });
}

fn fade_feed(
    feed: Res<Feed>,
    mut rows: Query<(&FeedRowNode, &mut BackgroundColor, &Children)>,
    mut paint: Query<(Option<&mut BackgroundColor>, Option<&mut TextColor>), Without<FeedRowNode>>,
) {
    for (row, mut background, children) in &mut rows {
        let Some(shown) = feed.0.get(row.0) else {
            continue;
        };
        let alpha = ((FEED_S - shown.age_s) / FEED_FADE_S).clamp(0.0, 1.0);
        background.0.set_alpha(ROW_ALPHA * alpha);
        for child in children.iter() {
            let Ok((background, text)) = paint.get_mut(child) else {
                continue;
            };
            if let Some(mut c) = background {
                c.0.set_alpha(alpha);
            }
            if let Some(mut c) = text {
                c.0.set_alpha(alpha);
            }
        }
    }
}

fn show_notice(
    mut commands: Commands,
    notice: Res<FullNotice>,
    (menu, settings): (Res<Menu>, Option<Res<Settings>>),
    mode: Option<Res<super::game_mode::Mode>>,
    nodes: Query<Entity, With<NoticeNode>>,
    mut shown: Local<Option<(String, bool)>>,
) {
    let survival = mode.is_some_and(|m| m.survival());
    let wanted = notice
        .text
        .clone()
        .filter(|_| in_play(&menu, settings.as_deref()))
        .map(|text| (text, survival));
    if *shown == wanted {
        return;
    }
    for node in &nodes {
        commands.entity(node).despawn();
    }
    *shown = wanted.clone();
    let Some((text, survival)) = wanted else {
        return;
    };
    commands
        .spawn((
            NoticeNode,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(super::hotbar::top_px(survival) + 4.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(11, 11, 13, 0xcc)),
                BorderColor::all(Color::srgba_u8(0xd6, 0xc8, 0x8f, 0x80)),
            ))
            .with_child((
                Text::new(text),
                TextFont {
                    weight: bevy::text::FontWeight(600),
                    ..font(12.0)
                },
                TextColor(Color::srgb_u8(0xea, 0xdf, 0xb4)),
            ));
        });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;
    use crate::content::ItemLook;

    /// Exact in binary, and within Bevy's default maximum frame delta.
    const FRAME: Duration = Duration::from_millis(250);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME))
            .insert_resource(Looks(vec![ItemLook {
                name: "t:rock".into(),
                size_m: 0.25,
                color: [1, 2, 3, 255],
                material: None,
                max_stack: 64,
            }]))
            .init_resource::<Menu>()
            .init_resource::<Feed>()
            .init_resource::<FullNotice>()
            .add_message::<Pickup>()
            .add_message::<InventoryChanged>()
            .add_systems(
                Update,
                (
                    receive_inventory,
                    receive_pickups,
                    age_feed,
                    show_feed,
                    fade_feed,
                    show_notice,
                )
                    .chain(),
            );
        app.update();
        app
    }

    fn picked(app: &mut App, item: &str, taken: u32, left: u32) {
        app.world_mut().write_message(Pickup {
            item: item.into(),
            taken,
            left,
        });
        app.update();
    }

    fn texts(world: &World, children: &Children) -> Vec<String> {
        children
            .iter()
            .filter_map(|c| world.get::<Text>(c).map(|t| t.0.clone()))
            .collect()
    }

    /// Each row's words, top first.
    fn feed(app: &mut App) -> Vec<String> {
        let world = app.world_mut();
        let Ok(root) = world
            .query_filtered::<&Children, With<FeedRoot>>()
            .single(world)
        else {
            return vec![];
        };
        root.iter()
            .map(|row| texts(world, world.get::<Children>(row).unwrap()).join(" "))
            .collect()
    }

    fn notice(app: &mut App) -> Option<String> {
        let world = app.world_mut();
        let node = world
            .query_filtered::<&Children, With<NoticeNode>>()
            .single(world)
            .ok()?;
        let card = node.iter().next()?;
        texts(world, world.get::<Children>(card)?).pop()
    }

    #[test]
    fn the_feed_puts_the_newest_on_top_merges_one_item_and_fades_after_three_seconds() {
        let mut app = app();
        picked(&mut app, "t:rock", 2, 0);
        picked(&mut app, "t:dust", 1, 0);
        assert_eq!(feed(&mut app), ["+1 t:dust", "+2 rock"]);
        picked(&mut app, "t:rock", 3, 0);
        assert_eq!(feed(&mut app), ["+5 rock", "+1 t:dust"]);
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(feed(&mut app), ["+5 rock"], "dust is 3 s old");
        app.update();
        assert!(feed(&mut app).is_empty());
    }

    #[test]
    fn a_refused_pickup_shows_the_notice_until_a_slot_frees() {
        let mut app = app();
        let mut held = Inventory::default();
        held.receive("t:dust", 64 * 36, 64);
        app.world_mut()
            .write_message(InventoryChanged(held.clone()));
        picked(&mut app, "t:rock", 0, 6);
        assert_eq!(
            notice(&mut app).as_deref(),
            Some("Inventory full · 6 rock left on the ground")
        );
        for _ in 0..10 {
            app.update();
        }
        assert!(notice(&mut app).is_some(), "it stays");
        let mut freed = Inventory::default();
        freed.receive("t:dust", 64 * 36 - 1, 64);
        app.world_mut().write_message(InventoryChanged(freed));
        app.update();
        assert_eq!(notice(&mut app), None);
    }

    #[test]
    fn a_pickup_that_takes_everything_shows_no_notice() {
        let mut app = app();
        picked(&mut app, "t:rock", 4, 0);
        assert_eq!(notice(&mut app), None);
    }
}
