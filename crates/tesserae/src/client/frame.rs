//! The menu frame every screen shares: a title over the scenery, the prompt
//! bar, focus first, hover that moves focus, and press feedback.
//!
//! A screen marks its root [`Frame`] and builds the rest from here; the
//! behaviour comes with the marker, so no screen wires it by hand.

use bevy::input::gamepad::GamepadButton;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::ui_widgets::{Activate, Button, TextInput};

use super::actions::{Device, Labels};
use super::menu_ui as ui;
use super::pad::PadInput;
use super::theme;

/// A screen's root. Hover, prompts and press feedback act only inside one.
#[derive(Component, Default)]
pub(super) struct Frame;

/// A control only the pointer reaches, such as the icons on a World's slot:
/// hover, Tab and the D-pad pass over it, and its keys are in the prompt bar.
#[derive(Component)]
pub(super) struct PointerOnly;

/// The control a [`Frame`] focuses when it opens; without one, its first.
#[derive(Component)]
pub(super) struct Autofocus;

/// A menu's popup, drawn over its screen by its `GlobalZIndex`. A press
/// anywhere outside it sends [`Dismiss`] to it and reaches nothing else.
#[derive(Component)]
pub(super) struct Popup;

/// A press landed outside this [`Popup`]; its screen closes it.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub(super) struct Dismiss {
    pub entity: Entity,
}

/// Covers the screen just under its popup, so the press that closes the popup
/// cannot also press a control under it.
#[derive(Component)]
struct Scrim(Entity);

/// Sent once for every press of a control in a [`Frame`], by key, click or
/// controller, as the action runs; the click sound and rumble hang off it.
#[derive(Event, Clone, Copy, Debug)]
pub(super) struct Press {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the click and rumble do not depend on which control"
        )
    )]
    pub entity: Entity,
    /// Made on a controller, so it rumbles.
    pub pad: bool,
}

/// How long a key or controller press shows; a pointer press shows while held.
const PRESS_S: f32 = 0.12;
const PRESS_SCALE: f32 = 0.97;
const PRESS_DARKEN: f32 = 0.06;

/// What a prompt's cap reads, and which key fires it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Cap {
    Tab,
    Enter,
    /// Moves a stepped screen on from anywhere on it; Ctrl or ⌘ both work.
    CtrlEnter,
    Esc,
    Del,
    Letter(char),
    /// A pointer gesture or a key range, named only: it fires nothing.
    Text(&'static str),
}

impl Cap {
    fn text(self) -> String {
        match self {
            Cap::Tab => "Tab".into(),
            Cap::Enter => "Enter".into(),
            Cap::CtrlEnter => "Ctrl+Enter".into(),
            Cap::Esc => "Esc".into(),
            Cap::Del => "Del".into(),
            Cap::Letter(c) => c.to_ascii_uppercase().to_string(),
            Cap::Text(text) => text.into(),
        }
    }

    /// Enter and Esc keep each screen's own handling, so only these fire here.
    fn keys(self) -> &'static [KeyCode] {
        const LETTERS: [KeyCode; 26] = [
            KeyCode::KeyA,
            KeyCode::KeyB,
            KeyCode::KeyC,
            KeyCode::KeyD,
            KeyCode::KeyE,
            KeyCode::KeyF,
            KeyCode::KeyG,
            KeyCode::KeyH,
            KeyCode::KeyI,
            KeyCode::KeyJ,
            KeyCode::KeyK,
            KeyCode::KeyL,
            KeyCode::KeyM,
            KeyCode::KeyN,
            KeyCode::KeyO,
            KeyCode::KeyP,
            KeyCode::KeyQ,
            KeyCode::KeyR,
            KeyCode::KeyS,
            KeyCode::KeyT,
            KeyCode::KeyU,
            KeyCode::KeyV,
            KeyCode::KeyW,
            KeyCode::KeyX,
            KeyCode::KeyY,
            KeyCode::KeyZ,
        ];
        match self {
            Cap::Del => &[KeyCode::Delete, KeyCode::Backspace],
            Cap::Letter(c) if c.is_ascii_alphabetic() => {
                let at = (c.to_ascii_uppercase() as u8 - b'A') as usize;
                std::slice::from_ref(&LETTERS[at])
            }
            Cap::Letter('/') => &[KeyCode::Slash],
            _ => &[],
        }
    }

    /// A is Enter and B is Esc everywhere; Del is the destructive Y.
    fn pad(self) -> Option<GamepadButton> {
        match self {
            Cap::Enter => Some(GamepadButton::South),
            Cap::CtrlEnter => Some(GamepadButton::Start),
            Cap::Esc => Some(GamepadButton::East),
            Cap::Del => Some(GamepadButton::North),
            Cap::Tab | Cap::Letter(_) | Cap::Text(_) => None,
        }
    }
}

/// One entry of the prompt bar: (key, label, act). With an act it is
/// clickable and answers its key; without one it only names the key.
pub(super) struct Prompt<A> {
    cap: Cap,
    label: String,
    act: Option<A>,
    pad: Option<GamepadButton>,
}

impl<A: Bundle> Prompt<A> {
    pub(super) fn new(cap: Cap, label: impl Into<String>) -> Self {
        Self {
            cap,
            label: label.into(),
            act: None,
            pad: cap.pad(),
        }
    }

    pub(super) fn act(self, act: A) -> Self {
        Self {
            act: Some(act),
            ..self
        }
    }

    /// The controller button that also fires it: X for a screen's secondary
    /// action, Y for its destructive or reset one.
    pub(super) fn pad(self, button: GamepadButton) -> Self {
        Self {
            pad: Some(button),
            ..self
        }
    }
}

/// A clickable prompt; it never takes focus.
#[derive(Component, Clone, Copy)]
pub(super) struct PromptKey {
    cap: Cap,
    pad: Option<GamepadButton>,
}

/// A prompt's cap, which names the key or the controller button.
#[derive(Component)]
struct CapText {
    key: String,
    pad: Option<GamepadButton>,
}

/// An item of a plain menu list, as the title screen's and the pause menu's.
#[derive(Component)]
#[require(Hovered, Outline = theme::focus_ring())]
pub(super) struct ListItem;

/// Restores what press feedback changed, before anything else reads it.
#[derive(Component)]
struct Squeezed(Option<Color>);

/// Shows press feedback until this time, for a press with no pointer held.
#[derive(Component)]
struct PressedUntil(f32);

#[derive(Default)]
struct FramePlugin;

impl Plugin for FramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputFocus>()
            .init_resource::<InputFocusVisible>()
            .init_resource::<Labels>()
            .add_observer(press)
            .add_observer(cover)
            .add_observer(uncover)
            .add_observer(dismiss)
            .add_observer(wheel_scroll)
            .add_systems(First, unsqueeze)
            .add_systems(
                PreUpdate,
                fire_prompts
                    .after(super::pad::read)
                    .after(bevy::input::InputSystems),
            )
            .add_systems(Update, (hover_focus, show_caps, paint_items))
            .add_systems(PostUpdate, (focus_first, squeeze));
    }
}

fn cover(add: On<Add<Popup>>, layers: Query<&GlobalZIndex>, mut commands: Commands) {
    let popup = add.entity;
    let Ok(&GlobalZIndex(z)) = layers.get(popup) else {
        warn!("popup {popup} has no GlobalZIndex, so nothing covers the screen under it");
        return;
    };
    commands.spawn((
        Scrim(popup),
        GlobalZIndex(z - 1),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    ));
}

fn uncover(remove: On<Remove<Popup>>, scrims: Query<(Entity, &Scrim)>, mut commands: Commands) {
    for (scrim, &Scrim(popup)) in &scrims {
        if popup == remove.entity {
            commands.entity(scrim).try_despawn();
        }
    }
}

fn dismiss(
    mut press: On<bevy::picking::events::PointerPress>,
    scrims: Query<&Scrim>,
    mut commands: Commands,
) {
    if let Ok(&Scrim(popup)) = scrims.get(press.entity) {
        press.propagate(false);
        commands.trigger(Dismiss { entity: popup });
    }
}

/// How far one wheel notch moves a list: about half a Mod card.
const WHEEL_LINE_PX: f32 = 48.0;

/// The wheel scrolls the innermost list under the pointer that has room to move.
fn wheel_scroll(
    mut wheel: On<bevy::picking::events::PointerScroll>,
    mut lists: Query<(&Node, &ComputedNode, &mut ScrollPosition)>,
    (parents, frames): (Query<&ChildOf>, Query<(), With<Frame>>),
) {
    let entity = wheel.entity;
    let Ok((node, computed, mut position)) = lists.get_mut(entity) else {
        return;
    };
    let room = (computed.content_size.y - computed.size.y + computed.scrollbar_size.y).max(0.0)
        * computed.inverse_scale_factor;
    if node.overflow.y != OverflowAxis::Scroll || room <= 0.0 || !framed(entity, &parents, &frames)
    {
        return;
    }
    wheel.propagate(false);
    let by = match wheel.unit {
        bevy::input::mouse::MouseScrollUnit::Line => wheel.y * WHEEL_LINE_PX,
        bevy::input::mouse::MouseScrollUnit::Pixel => wheel.y,
    };
    let y = (position.y - by).clamp(0.0, room);
    if position.y != y {
        position.y = y;
    }
}

/// Adds the frame's behaviour once, whichever of the screens using it asks first.
pub(super) fn plugin(app: &mut App) {
    if !app.is_plugin_added::<FramePlugin>() {
        app.add_plugins(FramePlugin);
    }
}

pub(super) const PROMPT_BAR_PX: f32 = 40.0;

/// The bottom bar: `place` at the left and the screen's prompts in the middle.
pub(super) fn prompt_bar<A: Bundle>(
    root: &mut ChildSpawnerCommands,
    place: &str,
    prompts: Vec<Prompt<A>>,
) {
    root.spawn((
        BackgroundColor(theme::band()),
        BorderColor::all(theme::band_rule()),
        Node {
            height: Val::Px(PROMPT_BAR_PX),
            flex_shrink: 0.0,
            padding: UiRect::horizontal(Val::Px(theme::SCREEN_PAD_X)),
            align_items: AlignItems::Center,
            border: UiRect::top(Val::Px(theme::RULE)),
            ..default()
        },
    ))
    .with_children(|bar| {
        bar.spawn(Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            ..default()
        })
        .with_child(chrome(place));
        bar.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|prompts_row| {
            for prompt in prompts {
                spawn_prompt(prompts_row, prompt);
            }
        });
        bar.spawn(Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            ..default()
        });
    });
}

/// A key's cap beside a control, as the pause menu's items and Respawn show it; it fires nothing.
pub(super) fn key_cap(parent: &mut ChildSpawnerCommands, key: &str, ink: Color) -> Entity {
    parent
        .spawn((
            BorderColor::all(ink.with_alpha(ink.alpha() * 0.5)),
            Node {
                padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                align_self: AlignSelf::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(key),
            theme::font(theme::TEXT_KEY),
            bevy::text::LetterSpacing::Px(0.9),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id()
}

fn chrome(value: &str) -> impl Bundle {
    (
        Text::new(value),
        theme::font(theme::TEXT_FOOTER),
        bevy::text::LetterSpacing::Px(0.9),
        TextColor(theme::chrome_ink()),
        Pickable::IGNORE,
    )
}

fn spawn_prompt<A: Bundle>(row: &mut ChildSpawnerCommands, prompt: Prompt<A>) {
    let node = Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    };
    let key = PromptKey {
        cap: prompt.cap,
        pad: prompt.pad,
    };
    let mut entity = row.spawn((node, BackgroundColor(Color::NONE)));
    if let Some(act) = prompt.act {
        entity.insert((Button, act, key, Hovered::default()));
    }
    entity.with_children(|p| {
        p.spawn((
            BorderColor::all(theme::key_edge()),
            Node {
                padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_child((
            CapText {
                key: prompt.cap.text(),
                pad: prompt.pad,
            },
            Text::new(prompt.cap.text()),
            theme::font(theme::TEXT_KEY),
            bevy::text::LetterSpacing::Px(0.9),
            TextColor(theme::chrome_ink()),
            Pickable::IGNORE,
        ));
        p.spawn(chrome(&prompt.label));
    });
}

/// A menu screen's sheet: its title over the scenery, an optional one-line
/// subtitle, and at most one main button. Back and every other action go in
/// the prompt bar.
pub(super) struct MenuSheet<'a, A> {
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub primary: Option<(&'a str, A)>,
    pub height: SheetHeight,
}

/// How tall a sheet stands in its row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum SheetHeight {
    /// As tall as its content, up to the row, scrolling inside past that.
    Content,
    /// The row's full height, so a stepped screen's main button stays put from step to step.
    Steps,
}

/// Spawns the sheet with `body` scrolling inside it, and returns the main button. `scroll` goes on
/// the scrolling body, for a screen that keeps its place there.
pub(super) fn menu_sheet<A: Bundle>(
    main: &mut ChildSpawnerCommands,
    sheet: MenuSheet<A>,
    scroll: impl Bundle,
    body: impl FnOnce(&mut ChildSpawnerCommands),
) -> Option<Entity> {
    let mut primary = None;
    let slot = &mut primary;
    let actions = sheet.primary.map(|(label, act)| {
        move |actions: &mut ChildSpawnerCommands| {
            *slot = Some(super::buttons::button(
                actions,
                label,
                act,
                super::buttons::ButtonStyle::Primary,
            ));
        }
    });
    sheet_with_actions(
        main,
        sheet.title,
        sheet.subtitle,
        sheet.height,
        scroll,
        body,
        actions,
    );
    primary
}

/// The sheet with its own row of buttons at the bottom, right-aligned unless `actions` lays
/// them out otherwise.
pub(super) fn sheet_with_actions(
    main: &mut ChildSpawnerCommands,
    title: &str,
    subtitle: Option<&str>,
    height: SheetHeight,
    scroll: impl Bundle,
    body: impl FnOnce(&mut ChildSpawnerCommands),
    actions: Option<impl FnOnce(&mut ChildSpawnerCommands)>,
) {
    main.spawn((
        BackgroundGradient(vec![
            LinearGradient::to_right(vec![
                ColorStop::new(theme::sheet_shade(), Val::Percent(0.0)),
                ColorStop::new(theme::sheet_shade_far(), Val::Percent(100.0)),
            ])
            .into(),
        ]),
        Node {
            flex_grow: 1.0,
            max_width: Val::Px(1180.0),
            min_height: Val::Px(0.0),
            max_height: Val::Percent(100.0),
            align_self: match height {
                SheetHeight::Content => AlignSelf::FlexStart,
                SheetHeight::Steps => AlignSelf::Stretch,
            },
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ))
    .with_children(|s| {
        s.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            padding: UiRect::new(Val::Px(32.0), Val::Px(32.0), Val::Px(28.0), Val::Px(8.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Text::new(title),
                theme::weighted(34.0, theme::STRONG),
                bevy::text::LetterSpacing::Px(-0.02 * 34.0),
                TextColor(theme::INK),
                Pickable::IGNORE,
            ));
            if let Some(subtitle) = subtitle {
                ui::paragraph(header, subtitle, theme::TEXT_SMALL, theme::muted());
            }
        });
        s.spawn((
            scroll,
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::new(Val::Px(32.0), Val::Px(32.0), Val::Px(16.0), Val::Px(24.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .with_children(body);
        if let Some(actions) = actions {
            s.spawn(Node {
                justify_content: JustifyContent::FlexEnd,
                padding: UiRect::new(Val::Px(32.0), Val::Px(32.0), Val::Px(0.0), Val::Px(24.0)),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(actions);
        }
    });
}

/// A plain menu list's item: the label, and an optional detail at the right
/// such as the World that Continue opens.
pub(super) fn list_item(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    detail: Option<&str>,
    act: impl Bundle,
) -> Entity {
    let shadow = TextShadow {
        offset: Vec2::new(0.0, 1.0),
        color: Color::BLACK.with_alpha(0.5),
    };
    parent
        .spawn((
            Button,
            bevy::input_focus::tab_navigation::TabIndex(0),
            act,
            ListItem,
            BackgroundColor(Color::NONE),
            Node {
                padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                column_gap: Val::Px(16.0),
                ..default()
            },
        ))
        .with_children(|item| {
            item.spawn((
                Text::new(label),
                theme::weighted(20.0, theme::LABEL),
                TextColor(theme::INK),
                shadow,
                Pickable::IGNORE,
            ));
            if let Some(detail) = detail {
                item.spawn((
                    Text::new(detail),
                    theme::font(13.0),
                    TextColor(theme::muted()),
                    shadow,
                    Pickable::IGNORE,
                ));
            }
        })
        .id()
}

type ItemPaint = (
    Entity,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut Outline,
    &'static Children,
);

/// One highlight: the focused item is lit, and hover moves focus to it.
fn paint_items(
    mut items: Query<ItemPaint, With<ListItem>>,
    mut texts: Query<&mut TextColor>,
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
) {
    let ringed = ui::visible_focus(Some(&focus), Some(&visible));
    for (entity, disabled, mut fill, mut outline, children) in &mut items {
        let lit = focus.get() == Some(entity);
        fill.set_if_neq(BackgroundColor(if lit {
            theme::LIT_ITEM
        } else {
            Color::NONE
        }));
        let ring = if ringed == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
        if let Some(label) = children.first()
            && let Ok(mut ink) = texts.get_mut(*label)
        {
            ink.set_if_neq(TextColor(if disabled {
                theme::muted()
            } else {
                theme::INK
            }));
        }
    }
}

fn framed(entity: Entity, parents: &Query<&ChildOf>, frames: &Query<(), With<Frame>>) -> bool {
    frames.contains(entity)
        || parents
            .iter_ancestors(entity)
            .any(|ancestor| frames.contains(ancestor))
}

type Focusable = (
    With<Button>,
    Without<PromptKey>,
    Without<PointerOnly>,
    Without<InteractionDisabled>,
);

fn focus_first(
    opened: Query<Entity, Added<Frame>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    autofocus: Query<(), With<Autofocus>>,
    focusable: Query<(), Focusable>,
    mut focus: ResMut<InputFocus>,
) {
    for frame in &opened {
        let inside = focus.get().is_some_and(|f| {
            f == frame || parents.iter_ancestors(f).any(|ancestor| ancestor == frame)
        });
        if inside {
            continue;
        }
        let order: Vec<Entity> = children
            .iter_descendants_depth_first(frame)
            .filter(|e| focusable.contains(*e))
            .collect();
        let first = order
            .iter()
            .find(|e| autofocus.contains(**e))
            .or(order.first());
        if let Some(entity) = first {
            focus.set(*entity, FocusCause::Auto);
        }
    }
}

/// As a mouse over a console menu does: moving onto an item focuses it. A
/// resting pointer never takes focus back from the keys.
fn hover_focus(
    motion: Option<Res<AccumulatedMouseMotion>>,
    hovered: Query<(Entity, &Hovered), Focusable>,
    typing: Query<(), With<TextInput>>,
    (parents, frames): (Query<&ChildOf>, Query<(), With<Frame>>),
    mut focus: ResMut<InputFocus>,
) {
    if motion.is_none_or(|m| m.delta == Vec2::ZERO) {
        return;
    }
    if focus.get().is_some_and(|f| typing.contains(f)) {
        return;
    }
    let target = hovered
        .iter()
        .filter(|(entity, hovered)| hovered.0 && framed(*entity, &parents, &frames))
        // An item inside another hovered one, such as a button on a card, wins.
        .max_by_key(|(entity, _)| parents.iter_ancestors(*entity).count())
        .map(|(entity, _)| entity);
    if let Some(entity) = target
        && focus.get() != Some(entity)
    {
        focus.set(entity, FocusCause::Navigated);
    }
}

type PromptQueries<'w, 's> = (
    Query<'w, 's, (Entity, &'static PromptKey), Without<InteractionDisabled>>,
    Query<'w, 's, &'static ChildOf>,
    Query<'w, 's, (), With<Frame>>,
    Query<'w, 's, (), With<TextInput>>,
    Query<'w, 's, (Entity, &'static bevy::input_focus::tab_navigation::TabGroup)>,
);

/// Letter and Delete prompts answer their key, and X and Y the prompts that
/// name them, in the frame that holds focus.
fn fire_prompts(
    mut commands: Commands,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pad: Option<Res<PadInput>>,
    focus: Res<InputFocus>,
    (prompts, parents, frames, typing, groups): PromptQueries,
) {
    let focused = focus.get();
    if focused.is_some_and(|f| typing.contains(f)) {
        return;
    }
    let modifier = keys.as_ref().is_some_and(|keys| {
        keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
            KeyCode::AltLeft,
            KeyCode::AltRight,
        ])
    });
    let ancestors = |e: Entity| std::iter::once(e).chain(parents.iter_ancestors(e));
    // A modal dialog over the screen answers its own keys.
    let modal = focused
        .and_then(|f| ancestors(f).find(|a| groups.get(*a).is_ok_and(|(_, group)| group.modal)));
    let frame_of = |e: Entity| ancestors(e).find(|a| frames.contains(*a));
    let focused_frame = focused.and_then(frame_of);
    for (entity, key) in &prompts {
        let Some(frame) = frame_of(entity) else {
            continue;
        };
        if focused_frame.is_some_and(|f| f != frame)
            || modal.is_some_and(|m| !ancestors(entity).any(|a| a == m))
        {
            continue;
        }
        let by_key = !modifier
            && keys
                .as_ref()
                .is_some_and(|keys| keys.any_just_pressed(key.cap.keys().iter().copied()));
        let by_pad = key
            .pad
            .filter(|b| matches!(b, GamepadButton::West | GamepadButton::North))
            .is_some_and(|b| pad.as_ref().is_some_and(|p| p.navigation.just_pressed(b)));
        if by_key || by_pad {
            commands.trigger(Activate { entity });
        }
    }
}

fn show_caps(labels: Res<Labels>, mut caps: Query<(&CapText, &mut Text)>) {
    for (cap, mut text) in &mut caps {
        let wanted = match cap.pad {
            Some(button) if labels.device == Device::Pad => labels.pad(button),
            _ => cap.key.clone(),
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

fn press(
    event: On<Activate>,
    mut commands: Commands,
    buttons: Query<(), With<Button>>,
    (parents, frames): (Query<&ChildOf>, Query<(), With<Frame>>),
    labels: Res<Labels>,
    time: Res<Time>,
) {
    let entity = event.entity;
    if !buttons.contains(entity) || !framed(entity, &parents, &frames) {
        return;
    }
    commands
        .entity(entity)
        .try_insert(PressedUntil(time.elapsed_secs() + PRESS_S));
    commands.trigger(Press {
        entity,
        pad: labels.device == Device::Pad,
    });
}

type Squeezable = (
    Entity,
    Has<Pressed>,
    Option<&'static PressedUntil>,
    Option<&'static mut BackgroundColor>,
    &'static mut UiTransform,
);

type BeingPressed = (Or<(With<Pressed>, With<PressedUntil>)>, With<Button>);

/// Shrinks and darkens a pressed control after every system has painted it;
/// [`unsqueeze`] undoes it first thing next frame, so painters never see it.
fn squeeze(
    mut commands: Commands,
    mut pressed: Query<Squeezable, BeingPressed>,
    (parents, frames): (Query<&ChildOf>, Query<(), With<Frame>>),
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for (entity, held, until, fill, mut transform) in &mut pressed {
        let timed = until.is_some_and(|u| now < u.0);
        if until.is_some() && !timed {
            commands.entity(entity).remove::<PressedUntil>();
        }
        if !(held || timed) || !framed(entity, &parents, &frames) {
            continue;
        }
        transform.scale = Vec2::splat(PRESS_SCALE);
        let original = fill.map(|mut fill| {
            let original = fill.0;
            fill.0 = original.darker(PRESS_DARKEN);
            original
        });
        commands.entity(entity).insert(Squeezed(original));
    }
}

fn unsqueeze(
    mut commands: Commands,
    mut squeezed: Query<(
        Entity,
        &Squeezed,
        Option<&mut BackgroundColor>,
        &mut UiTransform,
    )>,
) {
    for (entity, Squeezed(original), fill, mut transform) in &mut squeezed {
        transform.scale = Vec2::ONE;
        if let (Some(original), Some(mut fill)) = (original, fill) {
            fill.0 = *original;
        }
        commands.entity(entity).remove::<Squeezed>();
    }
}

/// Drives a frame the way a Player does, for any screen's tests.
#[cfg(test)]
pub(super) mod testing {
    use super::*;

    /// Moves the pointer onto `entity`, as a mouse does.
    pub(in crate::client) fn hover(app: &mut App, entity: Entity) {
        let all: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<Hovered>>()
            .iter(app.world())
            .collect();
        for e in all {
            app.world_mut().entity_mut(e).insert(Hovered(e == entity));
        }
        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = Vec2::new(12.0, 0.0);
        app.update();
        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = Vec2::ZERO;
    }

    /// The prompt bar as shown, in order: each prompt's cap and label.
    pub(in crate::client) fn prompts(app: &mut App) -> Vec<(String, String)> {
        let caps: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<CapText>>()
            .iter(app.world())
            .collect();
        let world = app.world();
        let parent = |e: Entity| world.get::<ChildOf>(e).unwrap().parent();
        let mut shown: Vec<(usize, String, String)> = caps
            .into_iter()
            .map(|cap| {
                let prompt = parent(parent(cap));
                let row = world.get::<Children>(parent(prompt)).unwrap();
                let at = row.iter().position(|e| e == prompt).unwrap();
                let label = world.get::<Children>(prompt).unwrap()[1];
                let text = |e: Entity| world.get::<Text>(e).unwrap().0.clone();
                (at, text(cap), text(label))
            })
            .collect();
        shown.sort();
        shown
            .into_iter()
            .map(|(_, cap, label)| (cap, label))
            .collect()
    }

    /// Whether `entity` shows press feedback now.
    pub(in crate::client) fn squeezed(app: &App, entity: Entity) -> bool {
        app.world()
            .get::<UiTransform>(entity)
            .is_some_and(|t| t.scale.x < 1.0)
    }

    /// Whether an open [`Popup`] covers the screen, swallowing a press outside it.
    pub(in crate::client) fn covered(app: &mut App) -> bool {
        app.world_mut()
            .query_filtered::<(), With<Scrim>>()
            .iter(app.world())
            .next()
            .is_some()
    }

    /// Presses outside every open [`Popup`], where the pointer meets its scrim.
    pub(in crate::client) fn click_outside(app: &mut App) {
        use bevy::picking::backend::HitData;
        use bevy::picking::events::{Pointer, PointerPress};
        use bevy::picking::pointer::{Location, PointerButton, PointerId};
        let scrims: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<Scrim>>()
            .iter(app.world())
            .collect();
        assert!(!scrims.is_empty(), "no popup is open");
        for scrim in scrims {
            app.world_mut().trigger(PointerPress {
                entity: scrim,
                pointer: Pointer::new(
                    PointerId::Mouse,
                    Location {
                        target: bevy::camera::NormalizedRenderTarget::None {
                            width: 800,
                            height: 600,
                        },
                        position: Vec2::ZERO,
                    },
                ),
                button: PointerButton::Primary,
                hit: HitData::new(scrim, 0.0, None, None),
                count: 1,
            });
        }
        app.update();
    }

    /// Turns the mouse wheel `lines` notches over `entity`; negative is down, as a wheel reports.
    pub(in crate::client) fn wheel(app: &mut App, entity: Entity, lines: f32) {
        use bevy::input::mouse::MouseScrollUnit;
        use bevy::input::touch::TouchPhase;
        use bevy::picking::backend::HitData;
        use bevy::picking::events::{Pointer, PointerScroll};
        use bevy::picking::pointer::{Location, PointerId};
        app.world_mut().trigger(PointerScroll {
            entity,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: Vec2::ZERO,
                },
            ),
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: lines,
            hit: HitData::new(entity, 0.0, None, None),
            phase: TouchPhase::Moved,
        });
        app.update();
    }

    /// What a [`Frame`] needs besides itself, in a headless test App.
    pub(in crate::client) fn support(app: &mut App) {
        app.init_resource::<AccumulatedMouseMotion>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<PadInput>();
        plugin(app);
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    #[derive(Component, Clone, Copy, PartialEq, Debug)]
    enum Act {
        Edit,
        Delete,
        Back,
        Play,
        Other,
    }

    #[derive(Resource, Default)]
    struct Fired(Vec<Act>);

    #[derive(Resource, Default)]
    struct Presses(Vec<Press>);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        support(&mut app);
        app.init_resource::<Fired>()
            .init_resource::<Presses>()
            .add_observer(
                |e: On<Activate>, acts: Query<&Act>, mut fired: ResMut<Fired>| {
                    if let Ok(act) = acts.get(e.entity) {
                        fired.0.push(*act);
                    }
                },
            )
            .add_observer(|e: On<Press>, mut presses: ResMut<Presses>| presses.0.push(*e));
        app
    }

    /// A framed screen: a field, two items and a bar of prompts.
    fn screen(app: &mut App, autofocus_second: bool) -> (Entity, Entity, Entity) {
        let mut ids = (
            Entity::PLACEHOLDER,
            Entity::PLACEHOLDER,
            Entity::PLACEHOLDER,
        );
        app.world_mut()
            .commands()
            .spawn((Frame, Node::default()))
            .with_children(|root| {
                ids.0 = root.spawn((Button, Act::Other, TextInput)).id();
                ids.1 = list_item(root, "Play", None, Act::Play);
                ids.2 = list_item(root, "Other", Some("detail"), Act::Other);
                if autofocus_second {
                    root.commands().entity(ids.2).insert(Autofocus);
                }
                prompt_bar(
                    root,
                    "HERE",
                    vec![
                        Prompt::new(Cap::Enter, "Play"),
                        Prompt::new(Cap::Letter('e'), "Edit")
                            .act(Act::Edit)
                            .pad(GamepadButton::West),
                        Prompt::new(Cap::Del, "Delete").act(Act::Delete),
                        Prompt::new(Cap::Esc, "Back").act(Act::Back),
                    ],
                );
            });
        app.update();
        ids
    }

    fn key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    fn pad(app: &mut App, button: GamepadButton) {
        app.world_mut().resource_mut::<Labels>().device = Device::Pad;
        app.world_mut()
            .resource_mut::<PadInput>()
            .navigation
            .press(button);
        app.update();
        let mut pad = app.world_mut().resource_mut::<PadInput>();
        pad.navigation.release(button);
        pad.navigation.clear();
    }

    fn fired(app: &mut App) -> Vec<Act> {
        std::mem::take(&mut app.world_mut().resource_mut::<Fired>().0)
    }

    fn focused(app: &App) -> Option<Entity> {
        app.world().resource::<InputFocus>().get()
    }

    fn prompt(app: &mut App, act: Act) -> Entity {
        app.world_mut()
            .query_filtered::<(Entity, &Act), With<PromptKey>>()
            .iter(app.world())
            .find_map(|(e, a)| (*a == act).then_some(e))
            .unwrap()
    }

    #[test]
    fn a_frame_opens_focused_on_its_first_item_or_the_one_it_names() {
        let mut app = app();
        let (field, _, _) = screen(&mut app, false);
        assert_eq!(focused(&app), Some(field));
        let mut app = self::app();
        let (_, _, named) = screen(&mut app, true);
        assert_eq!(focused(&app), Some(named));
    }

    #[test]
    fn letter_and_delete_prompts_answer_their_keys_and_clicks() {
        let mut app = app();
        let (_, play, _) = screen(&mut app, false);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(play, FocusCause::Navigated);
        key(&mut app, KeyCode::KeyE);
        key(&mut app, KeyCode::Delete);
        key(&mut app, KeyCode::Backspace);
        assert_eq!(fired(&mut app), [Act::Edit, Act::Delete, Act::Delete]);
        let edit = prompt(&mut app, Act::Edit);
        app.world_mut().trigger(Activate { entity: edit });
        assert_eq!(fired(&mut app), [Act::Edit]);
        key(&mut app, KeyCode::Escape);
        key(&mut app, KeyCode::Enter);
        assert!(
            fired(&mut app).is_empty(),
            "Esc and Enter keep their own handling"
        );
    }

    #[test]
    fn prompts_do_not_fire_while_a_text_field_has_focus_or_with_a_modifier() {
        let mut app = app();
        let (field, play, _) = screen(&mut app, false);
        assert_eq!(focused(&app), Some(field));
        key(&mut app, KeyCode::KeyE);
        key(&mut app, KeyCode::Backspace);
        assert!(fired(&mut app).is_empty());
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(play, FocusCause::Navigated);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::SuperLeft);
        key(&mut app, KeyCode::KeyE);
        assert!(fired(&mut app).is_empty(), "Cmd+E is not E");
    }

    #[test]
    fn x_and_y_fire_the_prompts_that_name_them_and_the_bar_shows_controller_buttons() {
        let mut app = app();
        let (_, play, _) = screen(&mut app, false);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(play, FocusCause::Navigated);
        let keys = [
            ("Enter".into(), "Play".into()),
            ("E".into(), "Edit".into()),
            ("Del".into(), "Delete".into()),
            ("Esc".into(), "Back".into()),
        ];
        assert_eq!(prompts(&mut app), keys);
        pad(&mut app, GamepadButton::West);
        pad(&mut app, GamepadButton::North);
        assert_eq!(fired(&mut app), [Act::Edit, Act::Delete]);
        let caps: Vec<String> = prompts(&mut app).into_iter().map(|(c, _)| c).collect();
        assert_eq!(caps, ["A", "X", "Y", "B"]);
        app.world_mut().resource_mut::<Labels>().device = Device::KeyboardMouse;
        app.update();
        assert_eq!(prompts(&mut app), keys);
    }

    #[test]
    fn moving_the_pointer_onto_an_item_moves_focus_and_a_resting_one_does_not() {
        let mut app = app();
        let (field, play, other) = screen(&mut app, false);
        hover(&mut app, other);
        assert_eq!(focused(&app), Some(field), "typing keeps its field");
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(play, FocusCause::Navigated);
        hover(&mut app, other);
        assert_eq!(focused(&app), Some(other));
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(play, FocusCause::Navigated);
        app.update();
        assert_eq!(
            focused(&app),
            Some(play),
            "a resting pointer keeps the keys' focus"
        );
        let edit = prompt(&mut app, Act::Edit);
        hover(&mut app, edit);
        assert_eq!(focused(&app), Some(play), "a prompt never takes focus");
    }

    #[test]
    fn a_press_shrinks_and_darkens_at_once_and_is_announced_once() {
        let mut app = app();
        let (_, play, _) = screen(&mut app, false);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(play, FocusCause::Navigated);
        app.update();
        let lit = app.world().get::<BackgroundColor>(play).unwrap().0;
        app.world_mut().trigger(Activate { entity: play });
        app.update();
        assert_eq!(fired(&mut app), [Act::Play], "the action is not delayed");
        assert!(squeezed(&app, play));
        let pressed = app.world().get::<BackgroundColor>(play).unwrap().0;
        assert!(
            pressed.to_linear().red < lit.to_linear().red,
            "{pressed:?} is not darker than {lit:?}"
        );
        let presses = std::mem::take(&mut app.world_mut().resource_mut::<Presses>().0);
        assert_eq!(presses.len(), 1);
        assert_eq!(presses[0].entity, play);
        assert!(!presses[0].pad);
        std::thread::sleep(std::time::Duration::from_secs_f32(PRESS_S * 1.5));
        app.update();
        app.update();
        assert!(!squeezed(&app, play));
        assert_eq!(app.world().get::<BackgroundColor>(play).unwrap().0, lit);
        app.world_mut().resource_mut::<Labels>().device = Device::Pad;
        app.world_mut().trigger(Activate { entity: play });
        app.update();
        assert!(app.world().resource::<Presses>().0[0].pad);
    }

    #[test]
    fn a_menu_sheet_shows_its_title_and_subtitle_and_at_most_one_main_button() {
        let mut app = app();
        let mut main = Entity::PLACEHOLDER;
        app.world_mut()
            .commands()
            .spawn((Frame, Node::default()))
            .with_children(|root| {
                main = menu_sheet(
                    root,
                    MenuSheet {
                        title: "Choose a Game",
                        subtitle: Some("It shapes the land."),
                        primary: Some(("Next", Act::Play)),
                        height: SheetHeight::Steps,
                    },
                    (),
                    |body| {
                        list_item(body, "Terra", None, Act::Other);
                    },
                )
                .unwrap();
            });
        app.update();
        let texts: Vec<String> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect();
        for wanted in ["Choose a Game", "It shapes the land.", "Next", "Terra"] {
            assert!(texts.iter().any(|t| t == wanted), "{wanted} in {texts:?}");
        }
        assert_eq!(app.world().get::<Act>(main), Some(&Act::Play));
        assert_eq!(
            focused(&app).and_then(|f| app.world().get::<Act>(f)),
            Some(&Act::Other)
        );
    }

    #[test]
    fn a_press_outside_a_popup_closes_it_and_reaches_nothing_under_it() {
        #[derive(Resource, Default)]
        struct Dismissed(Vec<Entity>);
        let mut app = app();
        app.init_resource::<Dismissed>()
            .add_observer(|e: On<Dismiss>, mut d: ResMut<Dismissed>| d.0.push(e.entity));
        screen(&mut app, false);
        let popup = app
            .world_mut()
            .spawn((Popup, GlobalZIndex(10), Node::default()))
            .id();
        app.update();
        let (z, node, pickable) = app
            .world_mut()
            .query_filtered::<(&GlobalZIndex, &Node, Option<&Pickable>), With<Scrim>>()
            .single(app.world())
            .map(|(z, n, p)| (z.0, n.clone(), p.cloned()))
            .unwrap();
        assert_eq!(z, 9, "just under the popup, over the screen");
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(
            (node.width, node.height),
            (Val::Percent(100.0), Val::Percent(100.0))
        );
        assert!(pickable.is_none_or(|p| p.should_block_lower && p.is_hoverable));
        click_outside(&mut app);
        assert_eq!(app.world().resource::<Dismissed>().0, [popup]);
        assert!(app.world().resource::<Fired>().0.is_empty());
        assert!(app.world().resource::<Presses>().0.is_empty());
        app.world_mut().entity_mut(popup).despawn();
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), With<Scrim>>()
                .iter(app.world())
                .count(),
            0,
            "the scrim goes with its popup"
        );
    }
}
