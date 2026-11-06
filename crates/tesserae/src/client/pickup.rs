//! What Pickup did, as the Server tells it: the feed at the bottom right and
//! the Inventory-full notice under the held Item's name. Drawn from Server
//! messages only (`docs/adr/0002-client-server-from-day-one.md`).

use bevy::prelude::*;
use tesserae_core::inventory::Inventory;

use super::drops::Looks;
use super::edit::Catalogue;
use super::inventory_window::{Names, item_icon};
use super::menu::Menu;
use super::menu_ui::declared_title;
use super::settings::Settings;
use super::theme;
use crate::server::entities::Pickup;
use crate::server::inventory::InventoryChanged;
use crate::session::SessionApp;

const FEED_S: f32 = 3.0;
const FEED_IN_S: f32 = 0.18;
const FEED_IN_PX: f32 = 16.0;
const FEED_ROWS: usize = 5;
const ICON_PX: f32 = 22.0;

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
/// Where the hotbar holds the Inventory-full notice, between the held Item's
/// name and the Vitals.
#[derive(Component)]
pub(super) struct FullNoticeAnchor;

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
                slide_feed,
                show_notice.after(super::hotbar::rebuild),
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

fn label(looks: &Looks, item: &str) -> String {
    declared_title(
        looks
            .0
            .iter()
            .find(|look| look.name == item)
            .map_or(item, |look| look.label()),
    )
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

/// The death screen hides the feed and the notice, as the Menu does.
fn in_play(menu: &Menu, settings: Option<&Settings>, dead: Option<Res<super::death::Dead>>) -> bool {
    !menu.open && !settings.is_some_and(|s| s.open) && super::death::alive(dead)
}

fn show_feed(
    mut commands: Commands,
    feed: Res<Feed>,
    (looks, catalogue): (Res<Looks>, Res<Catalogue>),
    (menu, settings, dead): (Res<Menu>, Option<Res<Settings>>, Option<Res<super::death::Dead>>),
    roots: Query<Entity, With<FeedRoot>>,
    mut shown: Local<Option<Vec<(String, u32)>>>,
) {
    let wanted = in_play(&menu, settings.as_deref(), dead).then(|| {
        feed.0
            .iter()
            .map(|r| (r.item.clone(), r.count))
            .collect::<Vec<_>>()
    });
    if *shown == wanted && !catalogue.is_changed() {
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
    let names = Names {
        looks: &looks,
        catalogue: &catalogue,
    };
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
                        border: UiRect::all(Val::Px(theme::FRAME)),
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
                        ..default()
                    },
                    BackgroundColor(theme::HUD_BAR),
                    BorderColor::all(theme::GROUT),
                    UiTransform::default(),
                    Pickable::IGNORE,
                ))
                .with_children(|row| {
                    row.spawn(theme::ring(theme::RADIUS_MENU))
                        .insert(BorderColor::all(theme::HUD_ROW_RING));
                    item_icon(row, names.icon(item), ICON_PX, false);
                    row.spawn((
                        Text::new(format!("+{count}")),
                        theme::weighted(theme::TEXT_CAPTION, theme::BOLD),
                        TextColor(theme::HUD_INK),
                        Pickable::IGNORE,
                    ));
                    row.spawn((
                        Text::new(label(&looks, item)),
                        theme::font(theme::TEXT_CAPTION),
                        TextColor(theme::HUD_QUIET_INK),
                        Pickable::IGNORE,
                    ));
                });
        }
        });
}

/// A new row slides in from the right; it leaves at once, as it came.
fn slide_feed(
    feed: Res<Feed>,
    preferences: Option<Res<super::profile::Preferences>>,
    mut rows: Query<(&FeedRowNode, &mut UiTransform)>,
) {
    let still = preferences.is_some_and(|p| p.reduced_motion);
    for (row, mut transform) in &mut rows {
        let Some(shown) = feed.0.get(row.0) else {
            continue;
        };
        let x = if still {
            0.0
        } else {
            let t = (shown.age_s / FEED_IN_S).clamp(0.0, 1.0);
            FEED_IN_PX * (1.0 - t).powi(2)
        };
        transform.set_if_neq(UiTransform::from_translation(Val2::px(x, 0.0)));
    }
}

fn show_notice(
    mut commands: Commands,
    notice: Res<FullNotice>,
    mut anchors: Query<(Entity, &mut Node, Option<&Children>), With<FullNoticeAnchor>>,
    nodes: Query<&Children, With<NoticeNode>>,
    texts: Query<&Text>,
) {
    for (anchor, mut node, children) in &mut anchors {
        let display = if notice.text.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let now = children
            .and_then(|c| c.iter().find_map(|c| nodes.get(c).ok()))
            .and_then(|c| c.iter().find_map(|c| texts.get(c).ok()))
            .map(|text| text.0.clone());
        if now == notice.text {
            continue;
        }
        commands.entity(anchor).despawn_related::<Children>();
        let Some(text) = notice.text.clone() else {
            continue;
        };
        commands.entity(anchor).with_child((
            NoticeNode,
            Node {
                padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                ..default()
            },
            BackgroundColor(theme::FULL_NOTICE),
            BorderColor::all(theme::FULL_NOTICE_EDGE),
            Pickable::IGNORE,
            children![(
                Text::new(text),
                theme::weighted(theme::TEXT_EYEBROW, theme::SEMIBOLD),
                TextColor(theme::FULL_NOTICE_INK),
                // `text-shadow: 0 1px 2px`, without the blur Bevy lacks.
                TextShadow {
                    offset: Vec2::new(0.0, 1.0),
                    color: theme::GROUT,
                },
                Pickable::IGNORE,
            )],
        ));
    }
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
            .init_resource::<Catalogue>()
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
                    slide_feed,
                    show_notice,
                )
                    .chain(),
            );
        app.world_mut().spawn((FullNoticeAnchor, Node::default()));
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
        let card = world
            .query_filtered::<&Children, With<NoticeNode>>()
            .single(world)
            .ok()?;
        texts(world, card).pop()
    }

    #[test]
    fn the_feed_puts_the_newest_on_top_merges_one_item_and_clears_after_three_seconds() {
        let mut app = app();
        picked(&mut app, "t:rock", 2, 0);
        picked(&mut app, "t:dust", 1, 0);
        assert_eq!(feed(&mut app), ["+1 Dust", "+2 Rock"]);
        picked(&mut app, "t:rock", 3, 0);
        assert_eq!(feed(&mut app), ["+5 Rock", "+1 Dust"]);
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(feed(&mut app), ["+5 Rock"], "dust is 3 s old");
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
            Some("Inventory full · 6 Rock left on the ground")
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
