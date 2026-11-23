//! Toasts: one message card for every screen, stacked where the prototype's
//! `/toasts` study put them: above the prompt bar in menus, under the Compass
//! in play.
use bevy::prelude::*;
use bevy::text::LineHeight;

use super::IconKind;
use crate::client::compass::CompassRoot;
use crate::client::input::{Owner, Ownership};
use crate::client::{frame, theme};
use crate::session::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::client) enum Tone {
    /// A setting or mode now reads differently.
    Info,
    /// Something the Player asked for took effect.
    Change,
    /// Something the Player asked for was refused.
    Alert,
    /// Something failed; it stays longer.
    Error,
}
impl Tone {
    fn seconds(self) -> f32 {
        match self {
            Self::Error => 8.0,
            _ => 3.0,
        }
    }
    fn icon(self) -> (IconKind, Color) {
        match self {
            Self::Info => (IconKind::Info, theme::TOAST_INFO),
            Self::Change => (IconKind::Done, theme::TOAST_DONE),
            Self::Alert | Self::Error => (IconKind::Warning, theme::WARN),
        }
    }
}

const MAX_SHOWN: usize = 3;
const MIN_WIDTH_PX: f32 = 220.0;
const MAX_WIDTH_PX: f32 = 420.0;
const SIDE_PX: f32 = 18.0;
/// Between the stack and the chrome it clears.
const GAP_PX: f32 = 10.0;
const ICON_PX: f32 = 16.0;
const COUNT_TEXT_PX: f32 = 11.0;
const FADE_S: f32 = 0.2;
const RISE_S: f32 = 0.22;
const RISE_PX: f32 = 8.0;
const BUMP_S: f32 = 0.24;
const COLLAPSE_DELAY_S: f32 = 0.04;
const LEAVE_S: f32 = 0.26;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Scope {
    #[default]
    Menus,
    Play,
}
impl Scope {
    fn text_px(self) -> f32 {
        match self {
            Self::Menus => theme::TEXT_CAPTION,
            Self::Play => theme::TEXT_SMALL,
        }
    }
}

#[derive(Component, Default)]
struct Stack {
    scope: Scope,
    /// Whether a focused frame has set `scope`; an unfocused one cannot tell a menu from play.
    settled: bool,
}
#[derive(Component)]
struct Column;
#[derive(Component)]
struct Toast {
    text: String,
    tone: Tone,
    count: u32,
    life: Timer,
    shown_s: f32,
    bump_s: Option<f32>,
    leaving_s: Option<f32>,
    /// The slot's height when it began to leave, which it collapses from.
    height_px: f32,
    count_label: Entity,
}
/// Something a toast fades, at its full colour.
#[derive(Component)]
struct Paint {
    toast: Entity,
    color: Color,
    part: Part,
}
/// Which of the painted node's colours `Paint` sets; every `Node` carries a fill and an edge.
#[derive(Clone, Copy)]
enum Part {
    Fill,
    FillAndEdge(Color),
    Ink,
    Image,
}
#[derive(Component)]
struct ToastText;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (place, age, fade).chain());
}

pub(in crate::client) fn notify(commands: &mut Commands, message: &str, tone: Tone) {
    notify_for(commands, message, tone, tone.seconds());
}

/// A message still showing counts up and restarts its clock instead of stacking a copy.
pub(in crate::client) fn notify_for(
    commands: &mut Commands,
    message: &str,
    tone: Tone,
    seconds: f32,
) {
    let message = message.to_owned();
    commands.queue(move |world: &mut World| show(world, message, tone, seconds));
}

fn show(world: &mut World, text: String, tone: Tone, seconds: f32) {
    let column = column(world);
    let live: Vec<Entity> = world
        .get::<Children>(column)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    let live: Vec<Entity> = live
        .into_iter()
        .filter(|e| {
            world
                .get::<Toast>(*e)
                .is_some_and(|t| t.leaving_s.is_none())
        })
        .collect();
    let life = Timer::from_seconds(seconds, TimerMode::Once);
    if let Some(&same) = live.iter().find(|e| {
        world
            .get::<Toast>(**e)
            .is_some_and(|t| t.text == text && t.tone == tone)
    }) {
        let mut toast = world.get_mut::<Toast>(same).unwrap();
        toast.count += 1;
        toast.life = life;
        toast.bump_s = Some(0.0);
        let (label, count) = (toast.count_label, toast.count);
        if let Some(mut node) = world.get_mut::<Node>(label) {
            node.display = Display::Flex;
        }
        let value = format!("\u{d7}{count}");
        match world
            .get::<Children>(label)
            .and_then(|c| c.first().copied())
        {
            Some(ink) => world.get_mut::<Text>(ink).unwrap().0 = value,
            None => {
                world.spawn((
                    ChildOf(label),
                    Paint {
                        toast: same,
                        color: theme::muted(),
                        part: Part::Ink,
                    },
                    Text::new(value),
                    theme::tabular(theme::font(COUNT_TEXT_PX)),
                    TextColor(Color::NONE),
                    Pickable::IGNORE,
                ));
            }
        }
        return;
    }
    for &oldest in live.iter().take((live.len() + 1).saturating_sub(MAX_SHOWN)) {
        leave(world, oldest);
    }
    let scope = world
        .get::<Stack>(stack_of(world, column))
        .map(|s| s.scope)
        .unwrap_or_default();
    let (kind, tone_color) = tone.icon();
    let mut commands = world.commands();
    let slot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            UiTransform::IDENTITY,
            Pickable::IGNORE,
        ))
        .id();
    let paint = |color: Color, part: Part| Paint {
        toast: slot,
        color,
        part,
    };
    let mut count_label = Entity::PLACEHOLDER;
    commands.entity(slot).with_children(|slot_children| {
        slot_children
            .spawn((
                paint(theme::toast_fill(), Part::FillAndEdge(theme::edge())),
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(9.0),
                    margin: UiRect::vertical(Val::Px(3.0)),
                    padding: UiRect {
                        left: Val::Px(12.0),
                        right: Val::Px(14.0),
                        top: Val::Px(9.0),
                        bottom: Val::Px(9.0),
                    },
                    border: UiRect::all(Val::Px(theme::RULE)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|card| {
                card.spawn((
                    paint(tone_color, Part::Image),
                    super::Icon(kind, Color::NONE),
                    Node {
                        width: Val::Px(ICON_PX),
                        height: Val::Px(ICON_PX),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                card.spawn((
                    ToastText,
                    paint(theme::NOTICE_CARD_INK, Part::Ink),
                    Text::new(text.clone()),
                    theme::font(scope.text_px()),
                    LineHeight::RelativeToFont(1.35),
                    TextColor(Color::NONE),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                count_label = card
                    .spawn((
                        paint(theme::TOAST_COUNT, Part::Fill),
                        BackgroundColor(Color::NONE),
                        Node {
                            display: Display::None,
                            flex_shrink: 0.0,
                            padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(9.0)),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
            });
    });
    commands.entity(slot).insert(Toast {
        text,
        tone,
        count: 1,
        life,
        shown_s: 0.0,
        bump_s: None,
        leaving_s: None,
        height_px: 0.0,
        count_label,
    });
    commands.entity(column).add_child(slot);
    world.flush();
}

fn leave(world: &mut World, slot: Entity) {
    let height_px = world
        .get::<ComputedNode>(slot)
        .map_or(0.0, |node| node.size.y * node.inverse_scale_factor);
    if let Some(mut toast) = world.get_mut::<Toast>(slot) {
        toast.leaving_s = Some(0.0);
        toast.height_px = height_px;
    }
}

fn stack_of(world: &World, column: Entity) -> Entity {
    world.get::<ChildOf>(column).map_or(column, ChildOf::parent)
}

fn column(world: &mut World) -> Entity {
    if let Some(column) = world
        .query_filtered::<Entity, With<Column>>()
        .iter(world)
        .next()
    {
        return column;
    }
    let mut column = Entity::PLACEHOLDER;
    world
        .spawn((
            Stack::default(),
            GlobalZIndex(40),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Px(frame::PROMPT_BAR_PX + GAP_PX),
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(Val::Px(SIDE_PX)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|stack| {
            column = stack
                .spawn((
                    Column,
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        min_width: Val::Px(MIN_WIDTH_PX),
                        max_width: Val::Px(MAX_WIDTH_PX),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
        });
    column
}

/// In play the HUD shows; every menu, the Inventory and the Map have a prompt bar.
fn scope_now(state: Option<&State<AppState>>, ownership: Option<&Ownership>) -> Option<Scope> {
    if state.is_none_or(|state| *state.get() != AppState::InWorld) {
        return Some(Scope::Menus);
    }
    match ownership.map_or(Owner::Gameplay, |o| o.owner) {
        Owner::Unfocused => None,
        Owner::Gameplay | Owner::PointerFree | Owner::Dead => Some(Scope::Play),
        _ => Some(Scope::Menus),
    }
}

fn place(
    state: Option<Res<State<AppState>>>,
    ownership: Option<Res<Ownership>>,
    dead: Option<Res<crate::client::death::Dead>>,
    compass: Query<(&ComputedNode, &UiGlobalTransform), With<CompassRoot>>,
    mut stacks: Query<(&mut Stack, &mut Node, &Children), Without<Column>>,
    mut columns: Query<&mut Node, With<Column>>,
    mut texts: Query<&mut TextFont, With<ToastText>>,
) {
    let Ok((mut stack, mut node, children)) = stacks.single_mut() else {
        return;
    };
    let display = if crate::client::death::alive(dead) {
        Display::Flex
    } else {
        Display::None
    };
    if node.display != display {
        node.display = display;
    }
    let scope = match scope_now(state.as_deref(), ownership.as_deref()) {
        Some(scope) => {
            stack.settled = true;
            scope
        }
        None if stack.settled => stack.scope,
        None => Scope::Play,
    };
    if stack.scope != scope {
        stack.scope = scope;
        for mut font in &mut texts {
            *font = theme::font(scope.text_px());
        }
    }
    let (top, bottom, direction) = match scope {
        Scope::Menus => (
            Val::Auto,
            Val::Px(frame::PROMPT_BAR_PX + GAP_PX),
            FlexDirection::Column,
        ),
        Scope::Play => {
            let under = compass
                .iter()
                .filter(|(node, _)| node.size.y > 0.0)
                .map(|(node, at)| {
                    (at.translation.y + node.size.y / 2.0) * node.inverse_scale_factor
                })
                .fold(0.0, f32::max);
            (
                Val::Px(under + GAP_PX),
                Val::Auto,
                FlexDirection::ColumnReverse,
            )
        }
    };
    if node.top != top || node.bottom != bottom {
        node.top = top;
        node.bottom = bottom;
    }
    for child in children {
        if let Ok(mut column) = columns.get_mut(*child)
            && column.flex_direction != direction
        {
            column.flex_direction = direction;
        }
    }
}

fn age(
    mut commands: Commands,
    time: Option<Res<Time>>,
    mut toasts: Query<(Entity, &mut Toast, &ComputedNode)>,
    mut fresh: Query<(Entity, &mut Toast), Without<ComputedNode>>,
) {
    let Some(time) = time else {
        return;
    };
    let dt = time.delta_secs();
    let mut tick = |entity: Entity, toast: &mut Toast, height_px: f32| {
        toast.shown_s += dt;
        if let Some(bump) = toast.bump_s.as_mut() {
            *bump += dt;
        }
        if let Some(leaving) = toast.leaving_s.as_mut() {
            *leaving += dt;
            if *leaving >= LEAVE_S {
                commands.entity(entity).despawn();
            }
        } else if toast.life.tick(time.delta()).is_finished() {
            toast.leaving_s = Some(0.0);
            toast.height_px = height_px;
        }
    };
    for (entity, mut toast, node) in &mut toasts {
        tick(entity, &mut toast, node.size.y * node.inverse_scale_factor);
    }
    for (entity, mut toast) in &mut fresh {
        tick(entity, &mut toast, 0.0);
    }
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// A toast's opacity: fades in, then out as it leaves.
fn opacity(toast: &Toast) -> f32 {
    let entered = ease_out(toast.shown_s / FADE_S);
    toast.leaving_s.map_or(entered, |leaving| {
        entered * (1.0 - leaving / FADE_S).max(0.0)
    })
}

type Painted = (
    &'static Paint,
    Option<&'static mut BackgroundColor>,
    Option<&'static mut BorderColor>,
    Option<&'static mut TextColor>,
    Option<&'static mut ImageNode>,
);

fn fade(
    preferences: Option<Res<crate::client::profile::Preferences>>,
    stacks: Query<&Stack>,
    mut slots: Query<(&Toast, &mut Node, &mut UiTransform)>,
    mut paints: Query<Painted>,
) {
    let still = preferences.is_some_and(|p| p.reduced_motion);
    let rise_px = match stacks.single().map(|s| s.scope).unwrap_or_default() {
        Scope::Menus => RISE_PX,
        Scope::Play => -RISE_PX,
    };
    for (toast, mut node, mut transform) in &mut slots {
        let mut wanted = UiTransform::IDENTITY;
        if !still {
            let rise = rise_px * (1.0 - ease_out(toast.shown_s / RISE_S));
            wanted.translation = Val2::px(0.0, if toast.leaving_s.is_some() { 0.0 } else { rise });
            if let Some(bump) = toast.bump_s.filter(|b| *b < BUMP_S) {
                let t = bump / BUMP_S;
                let peak = if t < 0.4 { t / 0.4 } else { (1.0 - t) / 0.6 };
                wanted.scale = Vec2::splat(1.0 + 0.03 * peak);
            }
        }
        if *transform != wanted {
            *transform = wanted;
        }
        let height = toast.leaving_s.map_or(Val::Auto, |leaving| {
            let t = ((leaving - COLLAPSE_DELAY_S) / (LEAVE_S - COLLAPSE_DELAY_S)).clamp(0.0, 1.0);
            Val::Px(toast.height_px * (1.0 - t * t))
        });
        if node.height != height {
            node.height = height;
        }
    }
    for (paint, fill, border, ink, image) in &mut paints {
        let Ok((toast, _, _)) = slots.get(paint.toast) else {
            continue;
        };
        let alpha = opacity(toast);
        let faded = |color: Color| color.with_alpha(color.alpha() * alpha);
        let color = faded(paint.color);
        match paint.part {
            Part::Fill | Part::FillAndEdge(_) => {
                if let Some(mut fill) = fill {
                    fill.0 = color;
                }
                if let (Part::FillAndEdge(edge), Some(mut border)) = (paint.part, border) {
                    *border = BorderColor::all(faded(edge));
                }
            }
            Part::Ink => {
                if let Some(mut ink) = ink {
                    ink.0 = color;
                }
            }
            Part::Image => {
                if let Some(mut image) = image {
                    image.color = color;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_systems(Update, (place, age, fade).chain());
        app
    }
    fn say(app: &mut App, message: &str, tone: Tone) {
        notify(&mut app.world_mut().commands(), message, tone);
        app.world_mut().flush();
        app.update();
    }
    fn wait(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(seconds));
        app.update();
        // A stale delta would age the toasts again on the next update.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::ZERO);
    }
    /// Each showing toast's message and its count label, oldest first.
    fn shown(app: &mut App) -> Vec<(String, String)> {
        let column = app
            .world_mut()
            .query_filtered::<Entity, With<Column>>()
            .single(app.world())
            .unwrap();
        let world = app.world();
        world
            .get::<Children>(column)
            .map(|c| c.iter().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|slot| world.get::<Toast>(slot))
            .filter(|toast| toast.leaving_s.is_none())
            .map(|toast| {
                let count = world
                    .get::<Children>(toast.count_label)
                    .map(|c| world.get::<Text>(c[0]).unwrap().0.clone())
                    .unwrap_or_default();
                (toast.text.clone(), count)
            })
            .collect()
    }

    #[test]
    fn the_same_message_again_counts_up_and_restarts_its_timer() {
        let mut app = app();
        say(&mut app, "Speed 2", Tone::Info);
        wait(&mut app, 2.0);
        say(&mut app, "Speed 2", Tone::Info);
        assert_eq!(shown(&mut app), [("Speed 2".into(), "\u{d7}2".into())]);
        wait(&mut app, 2.0);
        assert_eq!(shown(&mut app).len(), 1, "the repeat restarted the timer");
        wait(&mut app, 1.5);
        assert!(shown(&mut app).is_empty());
    }

    #[test]
    fn at_most_three_show_and_the_oldest_leaves_first() {
        let mut app = app();
        for message in ["One", "Two", "Three", "Four"] {
            say(&mut app, message, Tone::Change);
        }
        let texts: Vec<_> = shown(&mut app).into_iter().map(|(text, _)| text).collect();
        assert_eq!(texts, ["Two", "Three", "Four"]);
        wait(&mut app, LEAVE_S + 0.01);
        wait(&mut app, 0.0);
        assert_eq!(
            app.world_mut().query::<&Toast>().iter(app.world()).count(),
            3
        );
    }

    #[test]
    fn errors_stay_longer_than_refusals() {
        let mut app = app();
        say(&mut app, "Backpack full", Tone::Alert);
        say(&mut app, "Could not save the World", Tone::Error);
        wait(&mut app, 3.5);
        let texts: Vec<_> = shown(&mut app).into_iter().map(|(text, _)| text).collect();
        assert_eq!(texts, ["Could not save the World"]);
        wait(&mut app, 5.0);
        assert!(shown(&mut app).is_empty());
    }

    #[test]
    fn tone_sets_only_the_icon_colour() {
        let mut app = app();
        say(&mut app, "Speed 2", Tone::Info);
        say(&mut app, "Seed copied", Tone::Change);
        say(&mut app, "Backpack full", Tone::Alert);
        wait(&mut app, 1.0);
        let cards: Vec<(Color, BorderColor)> = app
            .world_mut()
            .query_filtered::<(&BackgroundColor, &BorderColor), Without<Text>>()
            .iter(app.world())
            .filter(|(_, border)| border.top != Color::NONE)
            .map(|(fill, border)| (fill.0, *border))
            .collect();
        assert_eq!(cards.len(), 3);
        let filled = app
            .world_mut()
            .query::<&BackgroundColor>()
            .iter(app.world())
            .filter(|fill| fill.0 != Color::NONE)
            .count();
        assert_eq!(filled, 6, "only cards and count bubbles are filled");
        for card in &cards {
            assert_eq!(
                *card,
                (theme::toast_fill(), BorderColor::all(theme::edge()))
            );
        }
        let icons: Vec<(IconKind, Color)> = app
            .world_mut()
            .query::<(&super::super::Icon, &Paint)>()
            .iter(app.world())
            .map(|(icon, paint)| (icon.0, paint.color))
            .collect();
        assert_eq!(
            icons,
            [
                (IconKind::Info, theme::TOAST_INFO),
                (IconKind::Done, theme::TOAST_DONE),
                (IconKind::Warning, theme::WARN),
            ]
        );
    }

    fn stack(app: &mut App) -> (Node, Node, Vec<f32>) {
        let (node, children) = app
            .world_mut()
            .query_filtered::<(&Node, &Children), With<Stack>>()
            .single(app.world())
            .unwrap();
        let (node, column) = (node.clone(), children[0]);
        let sizes = app
            .world_mut()
            .query_filtered::<&TextFont, With<ToastText>>()
            .iter(app.world())
            .map(|font| match font.font_size {
                bevy::text::FontSize::Px(px) => px,
                _ => 0.0,
            })
            .collect();
        (
            node,
            app.world().get::<Node>(column).unwrap().clone(),
            sizes,
        )
    }

    #[test]
    fn menus_stack_above_the_prompt_bar_in_menu_text() {
        let mut app = app();
        app.insert_resource(State::new(AppState::List));
        say(&mut app, "Seed copied", Tone::Change);
        let (node, column, sizes) = stack(&mut app);
        assert_eq!(node.bottom, Val::Px(50.0));
        assert_eq!(node.top, Val::Auto);
        assert_eq!(column.flex_direction, FlexDirection::Column);
        assert_eq!(sizes, [13.0]);
    }

    #[test]
    fn play_stacks_under_the_compass_newest_on_top_in_larger_text() {
        let mut app = app();
        app.insert_resource(State::new(AppState::InWorld))
            .insert_resource(Ownership::playing());
        let compass = ComputedNode {
            size: Vec2::new(1120.0, 160.0),
            inverse_scale_factor: 0.5,
            ..default()
        };
        app.world_mut().spawn((
            CompassRoot,
            compass,
            UiGlobalTransform::from_translation(Vec2::new(1280.0, 108.0)),
        ));
        say(&mut app, "Speed 2", Tone::Info);
        let (node, column, sizes) = stack(&mut app);
        assert_eq!(node.top, Val::Px(94.0 + GAP_PX));
        assert_eq!(column.flex_direction, FlexDirection::ColumnReverse);
        assert_eq!(sizes, [14.0]);

        let mut menu = Ownership::playing();
        menu.owner = Owner::Inventory;
        app.insert_resource(menu);
        app.update();
        let (node, _, sizes) = stack(&mut app);
        assert_eq!(node.bottom, Val::Px(50.0));
        assert_eq!(sizes, [13.0]);
    }

    #[test]
    fn an_unfocused_world_window_places_a_new_stack_for_play() {
        let mut app = app();
        let mut unfocused = Ownership::playing();
        unfocused.owner = Owner::Unfocused;
        app.insert_resource(State::new(AppState::InWorld))
            .insert_resource(unfocused);
        say(&mut app, "Speed 2", Tone::Info);
        let (node, _, sizes) = stack(&mut app);
        assert_eq!(node.top, Val::Px(GAP_PX));
        assert_eq!(sizes, [14.0]);
    }

    #[test]
    fn reduced_motion_fades_without_rising() {
        let mut app = app();
        app.insert_resource(crate::client::profile::Preferences {
            reduced_motion: true,
            ..default()
        });
        say(&mut app, "Saved", Tone::Change);
        wait(&mut app, 0.05);
        let transform = *app
            .world_mut()
            .query_filtered::<&UiTransform, With<Toast>>()
            .single(app.world())
            .unwrap();
        assert_eq!(transform, UiTransform::IDENTITY);
        let ink = app
            .world_mut()
            .query_filtered::<&TextColor, With<ToastText>>()
            .single(app.world())
            .unwrap()
            .0;
        assert!(
            0.0 < ink.alpha() && ink.alpha() < 1.0,
            "{ink:?} is mid-fade"
        );
    }

    #[test]
    fn a_stack_shares_the_widest_cards_width_and_is_at_least_220_px() {
        let mut app = layout_app();
        say(&mut app, "Speed 2", Tone::Info);
        // Without a renderer the camera's viewport stays unsized, so 100% of it is nothing.
        let mut root = app
            .world_mut()
            .query_filtered::<&mut Node, With<Stack>>()
            .single_mut(app.world_mut())
            .unwrap();
        root.width = Val::Px(1280.0);
        for _ in 0..3 {
            app.update();
        }
        let narrow = card_widths(&mut app);
        assert_eq!(narrow.len(), 1);
        assert!((narrow[0] - MIN_WIDTH_PX).abs() < 0.5, "{narrow:?}");
        say(
            &mut app,
            "Could not save the World because the disk is full",
            Tone::Error,
        );
        for _ in 0..3 {
            app.update();
        }
        let widths = card_widths(&mut app);
        assert_eq!(widths.len(), 2);
        assert!(widths[0] > MIN_WIDTH_PX + 50.0, "{widths:?}");
        assert!((widths[0] - widths[1]).abs() < 0.5, "{widths:?}");
    }

    fn card_widths(app: &mut App) -> Vec<f32> {
        app.world_mut()
            .query_filtered::<(&ComputedNode, &BorderColor), Without<Text>>()
            .iter(app.world())
            .filter(|(_, border)| border.top != Color::NONE)
            .map(|(node, _)| node.size.x * node.inverse_scale_factor)
            .collect()
    }

    fn layout_app() -> App {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::transform::TransformPlugin,
            bevy::input::InputPlugin,
            bevy::a11y::AccessibilityPlugin,
            bevy::picking::DefaultPickingPlugins,
            bevy::input_focus::InputFocusPlugin,
            bevy::window::WindowPlugin::default(),
            bevy::image::ImagePlugin::default(),
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
        ));
        app.add_message::<bevy::window::WindowEvent>()
            .add_message::<bevy::picking::backend::PointerHits>()
            .init_asset::<Image>()
            .init_asset::<bevy::image::TextureAtlasLayout>()
            .init_asset::<Mesh>()
            .init_asset::<bevy::shader::Shader>()
            .add_systems(Update, (place, age, fade).chain());
        app.world_mut()
            .resource_mut::<Assets<Font>>()
            .insert(
                bevy::asset::AssetId::default(),
                Font::from_bytes(
                    include_bytes!("../../../../../assets/fonts/InterVariable.ttf").to_vec(),
                ),
            )
            .expect("the default Font asset can be replaced");
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        app.world_mut().spawn(Camera2d);
        app
    }
}
