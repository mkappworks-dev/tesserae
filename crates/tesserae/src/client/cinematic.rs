//! The approved menu look's building blocks: framed panels and sheets, the
//! button states, notices and pills, all drawn from [`theme`].

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

use super::menu_ui as ui;
use super::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Primary,
    Plain,
    Ghost,
    Danger,
}

/// A button; [`paint`] keeps its colours in step with hover, focus and
/// disabling.
#[derive(Component, Clone, Copy, Debug)]
#[require(Hovered, Outline = theme::focus_ring())]
pub(super) struct Look {
    pub kind: Kind,
    pub selected: bool,
}

/// A button label whose ink [`paint`] keeps in step with the button's state.
#[derive(Component)]
pub(super) struct Label;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, paint.before(bevy::ui::UiSystems::Layout));
}

/// Fill, edge and label ink for a button in one state.
pub(super) fn colours(look: Look, lit: bool, disabled: bool) -> (Color, Color, Color) {
    match look.kind {
        Kind::Primary if disabled => (
            theme::PRIMARY_DISABLED,
            theme::PRIMARY_DISABLED_EDGE,
            theme::PRIMARY_DISABLED_INK,
        ),
        _ if disabled => (theme::DISABLED, theme::DISABLED_EDGE, theme::DISABLED_INK),
        Kind::Primary => (
            if lit { theme::SAGE_HOVER } else { theme::SAGE },
            theme::SAGE,
            theme::SAGE_INK,
        ),
        _ if look.selected => (theme::CHOSEN, theme::CHOSEN_EDGE, theme::INK),
        Kind::Danger => (
            if lit {
                theme::DANGER_HOVER
            } else {
                theme::DANGER
            },
            if lit {
                theme::DANGER_HOVER
            } else {
                theme::DANGER_EDGE
            },
            theme::DANGER_INK,
        ),
        _ if lit => (theme::HOVER, theme::edge(), theme::INK),
        Kind::Plain => (theme::RAISED, theme::edge(), theme::INK),
        Kind::Ghost => (theme::GHOST, theme::edge(), theme::INK),
    }
}

type Painted = (
    Entity,
    &'static Look,
    &'static Hovered,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut Outline,
    Option<&'static Children>,
);

fn paint(
    mut buttons: Query<Painted>,
    mut labels: Query<&mut TextColor, With<Label>>,
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
) {
    let focused = ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, look, hovered, disabled, mut background, mut border, mut outline, children) in
        &mut buttons
    {
        let (fill, edge, ink) = colours(*look, hovered.0 && !disabled, disabled);
        background.set_if_neq(BackgroundColor(fill));
        border.set_if_neq(BorderColor::all(edge));
        let ring = if focused == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
        for child in children.into_iter().flatten() {
            if let Ok(mut text) = labels.get_mut(*child) {
                text.set_if_neq(TextColor(ink));
            }
        }
    }
}

/// A framed panel: a black frame, the light ring inside it and a deep shadow.
pub(super) fn surface(background: Color) -> impl Bundle {
    (
        BackgroundColor(background),
        BorderColor::all(theme::GROUT),
        theme::shadow(),
        children![theme::ring(theme::RADIUS_PANEL)],
    )
}

/// The node of a framed panel, given its own layout.
pub(super) fn framed(node: Node) -> Node {
    Node {
        border: UiRect::all(Val::Px(theme::FRAME)),
        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
        ..node
    }
}

/// A modal dialog's box, centred over [`theme::overlay()`].
pub(super) fn dialog_box() -> impl Bundle {
    (
        surface(theme::panel()),
        framed(Node {
            width: Val::Px(420.0),
            max_width: Val::Percent(92.0),
            padding: UiRect::all(Val::Px(25.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            ..default()
        }),
    )
}

pub(super) fn dialog_title(parent: &mut ChildSpawnerCommands, value: &str) {
    ui::weighted_text(
        parent,
        value,
        theme::TEXT_DIALOG_TITLE,
        theme::MEDIUM,
        theme::INK,
    );
}

pub(super) fn dialog_text(
    parent: &mut ChildSpawnerCommands,
    value: impl Into<String>,
    color: Color,
) {
    parent.spawn((
        Text::new(value),
        theme::font(theme::TEXT_EYEBROW),
        bevy::text::LineHeight::RelativeToFont(1.8),
        TextColor(color),
        Pickable::IGNORE,
    ));
}

/// A dialog's buttons, at its bottom right.
pub(super) fn dialog_actions() -> Node {
    Node {
        justify_content: JustifyContent::FlexEnd,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(8.0),
        flex_wrap: FlexWrap::Wrap,
        margin: UiRect::top(Val::Px(11.0)),
        ..default()
    }
}

/// An error well, as the prototype's validation summary draws it.
pub(super) fn error_well() -> impl Bundle {
    (
        BackgroundColor(theme::DANGER),
        BorderColor::all(theme::DANGER_EDGE),
        Node {
            padding: UiRect::all(Val::Px(12.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
            ..default()
        },
    )
}

/// A quiet note with a rule down its left side.
pub(super) fn notice(parent: &mut ChildSpawnerCommands, value: impl Into<String>) {
    parent
        .spawn((
            BackgroundColor(theme::NOTICE),
            BorderColor::all(theme::notice_edge()),
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(9.0)),
                border: UiRect::left(Val::Px(theme::FRAME)),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|n| {
            // Tabular so the despawn countdown does not jitter as it ticks.
            n.spawn((
                Text::new(value.into()),
                theme::tabular(theme::font(theme::TEXT_SMALL)),
                bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        });
}

/// A small outlined tag, in capitals.
pub(super) fn pill(parent: &mut ChildSpawnerCommands, value: impl Into<String>) {
    parent
        .spawn((
            BackgroundColor(theme::PILL),
            BorderColor::all(theme::PILL_EDGE),
            Node {
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(value.into().to_uppercase()),
                theme::font(theme::TEXT_CAPTION),
                bevy::text::LetterSpacing::Px(0.78),
                bevy::text::LineHeight::RelativeToFont(1.0),
                TextColor(theme::PILL_INK),
                Pickable::IGNORE,
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_primary_reads_as_unavailable_not_as_a_faded_primary() {
        let look = Look {
            kind: Kind::Primary,
            selected: false,
        };
        let (fill, edge, ink) = colours(look, false, true);
        assert_eq!(
            (fill, edge, ink),
            (
                theme::PRIMARY_DISABLED,
                theme::PRIMARY_DISABLED_EDGE,
                theme::PRIMARY_DISABLED_INK
            )
        );
        assert_eq!(colours(look, true, true).0, theme::PRIMARY_DISABLED);
    }

    #[test]
    fn hover_lifts_every_kind_but_selection_holds() {
        for kind in [Kind::Plain, Kind::Ghost] {
            let look = Look {
                kind,
                selected: false,
            };
            assert_eq!(colours(look, true, false).0, theme::HOVER, "{kind:?}");
        }
        let danger = Look {
            kind: Kind::Danger,
            selected: false,
        };
        assert_eq!(colours(danger, false, false).0, theme::DANGER);
        assert_eq!(colours(danger, true, false).0, theme::DANGER_HOVER);
        let chosen = Look {
            kind: Kind::Plain,
            selected: true,
        };
        assert_eq!(colours(chosen, true, false).0, theme::CHOSEN);
    }
}
