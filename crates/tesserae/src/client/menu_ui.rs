//! Shared menu chrome; page bodies never replace the focused search widget.
use super::settings::{Operation, Settings};
use bevy::{
    input_focus::{FocusCause, InputFocus, InputFocusSystems, tab_navigation::TabIndex},
    prelude::*,
    text::{EditableText, TextEdit},
    ui_widgets::{Activate, Button, TextInput},
};

pub(super) const LEAF: Color = Color::srgb(0.286745, 0.696314, 0.384471);
pub(super) const BG: Color = Color::srgb(10.0 / 255.0, 10.0 / 255.0, 10.0 / 255.0);
pub(super) const CARD: Color = Color::srgb(17.0 / 255.0, 17.0 / 255.0, 17.0 / 255.0);
pub(super) const EDGE: Color = Color::srgb(25.0 / 255.0, 25.0 / 255.0, 25.0 / 255.0);
pub(super) const INK: Color = Color::srgb(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0);
pub(super) const MUTED: Color = Color::srgb(129.0 / 255.0, 129.0 / 255.0, 129.0 / 255.0);

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(super) enum Tab {
    Catalogue,
    #[default]
    Behavior,
    Bindings,
    Display,
}
impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Catalogue => "Catalogue",
            Self::Behavior => "Behavior",
            Self::Bindings => "Key bindings",
            Self::Display => "Display",
        }
    }
}
#[derive(Resource, Default)]
pub(super) struct Searches {
    pub catalogue: String,
    pub bindings: String,
}
pub(super) fn title(name: &str) -> String {
    let name = name.rsplit(':').next().unwrap_or(name);
    let mut letters = name.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + letters.as_str()
    })
}
pub(super) fn matches(query: &str, text: &str) -> bool {
    let text = text.to_lowercase();
    query
        .split_whitespace()
        .all(|word| text.contains(&word.to_lowercase()))
}
#[derive(Component)]
struct Shell;
#[derive(Resource)]
struct Mosaic(Handle<Image>);
#[derive(Resource)]
struct Fonts {
    _handles: Vec<Handle<Font>>,
}
#[derive(Component)]
pub(super) struct SearchField(Tab);
#[derive(Component)]
struct ClearSearch;
#[derive(Component)]
struct Quiet;
#[derive(Component)]
pub(super) struct BindingControl;
#[derive(Component)]
pub(super) struct Segment;
#[derive(Component)]
struct SearchPlaceholder;
#[derive(Component)]
struct SearchSlash;
#[derive(Component)]
struct SearchFrame;
#[derive(Component)]
struct SearchCount(Tab);
#[derive(Component)]
struct HeadingPill(Tab);
#[derive(Component)]
pub(super) struct Styled {
    pub selected: bool,
    pub tab: bool,
    pub primary: bool,
}

pub(super) fn font(size: f32) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::family("Liberation Sans"),
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}
pub(super) fn text(
    parent: &mut ChildSpawnerCommands,
    value: impl Into<String>,
    size: f32,
    color: Color,
) {
    parent.spawn((
        Text::new(value),
        font(size),
        TextColor(color),
        bevy::text::LineHeight::RelativeToFont(1.5),
        Pickable::IGNORE,
    ));
}
pub(super) fn button(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: impl Bundle,
    selected: bool,
    tab: bool,
    primary: bool,
) -> Entity {
    parent
        .spawn((
            Button,
            TabIndex(0),
            operation,
            Styled {
                selected,
                tab,
                primary,
            },
            Node {
                min_height: Val::Px(39.0),
                padding: if tab {
                    UiRect {
                        left: Val::Px(2.0),
                        right: Val::Px(2.0),
                        top: Val::Px(8.0),
                        bottom: Val::Px(11.0),
                    }
                } else {
                    UiRect::axes(Val::Px(13.0), Val::Px(9.0))
                },
                border: if tab {
                    UiRect::bottom(Val::Px(2.0))
                } else {
                    UiRect::all(Val::Px(1.0))
                },
                border_radius: BorderRadius::all(Val::Px(if tab { 0.0 } else { 5.0 })),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(if primary {
                LEAF
            } else if tab {
                Color::NONE
            } else {
                CARD
            }),
            BorderColor::all(if selected || primary {
                LEAF
            } else if tab {
                Color::NONE
            } else {
                EDGE
            }),
        ))
        .with_child((
            Text::new(label),
            TextFont {
                weight: bevy::text::FontWeight(if primary { 650 } else { 400 }),
                ..font(12.0)
            },
            bevy::text::LineHeight::RelativeToFont(1.5),
            TextColor(if primary {
                BG
            } else if tab && !selected {
                MUTED
            } else {
                INK
            }),
            Pickable::IGNORE,
        ))
        .id()
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Searches>()
        .init_resource::<InputFocus>()
        .add_observer(clear_search)
        .add_systems(Startup, (make_logo, load_fonts, make_icons))
        .add_systems(
            PostUpdate,
            materialize_icons.before(bevy::ui::UiSystems::Layout),
        )
        .add_systems(
            Update,
            (
                search_presentation,
                heading_status,
                button_label_colors,
                age_notifications,
            )
                .after(shell),
        )
        .add_systems(
            PreUpdate,
            focus
                .after(InputFocusSystems::Dispatch)
                .after(bevy::picking::PickingSystems::Last),
        )
        .add_systems(
            Update,
            (search_keys, sync_search)
                .chain()
                .before(super::menu::toggle)
                .before(super::settings::capture),
        )
        .add_systems(
            Update,
            (shell, style).chain().after(super::settings::render),
        );
}
fn focus(
    focus: Res<InputFocus>,
    fields: Query<(), With<SearchField>>,
    mut ui: ResMut<super::input::UiInput>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    ui.text_entry = focus.get().is_some_and(|e| fields.contains(e))
        || (ui.text_entry && keys.just_pressed(KeyCode::Escape) && !fields.is_empty());
}
fn clear_search(
    event: On<Activate>,
    clear: Query<(), With<ClearSearch>>,
    mut fields: Query<(Entity, &mut EditableText), With<SearchField>>,
    mut focus: ResMut<InputFocus>,
) {
    if !clear.contains(event.entity) {
        return;
    }
    for (entity, mut text) in &mut fields {
        text.queue_edit(TextEdit::SelectAll);
        text.queue_edit(TextEdit::Insert("".into()));
        focus.set(entity, FocusCause::Pressed);
    }
}
fn search_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<InputFocus>,
    mut fields: Query<(Entity, &mut EditableText), With<SearchField>>,
    mut ui: ResMut<super::input::UiInput>,
    mut actions: ResMut<super::actions::Actions>,
) {
    if keys.just_pressed(KeyCode::Escape) && ui.text_entry {
        actions.consume_cancel();
        for (entity, mut text) in &mut fields {
            if !text.value().to_string().is_empty() {
                text.queue_edit(TextEdit::SelectAll);
                text.queue_edit(TextEdit::Insert("".into()));
                focus.set(entity, FocusCause::Navigated);
            } else {
                focus.clear();
            }
        }
        // Keep ownership for this frame, including the Escape that ends editing.
        ui.text_entry = true;
    } else if keys.just_pressed(KeyCode::Slash)
        && !ui.text_entry
        && let Some((entity, _)) = fields.iter().next()
    {
        focus.set(entity, FocusCause::Navigated);
        ui.text_entry = true;
    }
}
type SearchDecorations<'w, 's> = Query<
    'w,
    's,
    (&'static mut Visibility, &'static mut Node, Has<ClearSearch>),
    Or<(
        With<SearchPlaceholder>,
        With<ClearSearch>,
        With<SearchSlash>,
    )>,
>;
fn sync_search(
    fields: Query<(&SearchField, &EditableText)>,
    mut searches: ResMut<Searches>,
    mut settings: ResMut<Settings>,
    mut decorations: SearchDecorations,
) {
    if let Some((field, text)) = fields.iter().next() {
        let value = text.value().to_string();
        for (mut visibility, mut node, clear) in &mut decorations {
            let show = if clear {
                !value.is_empty()
            } else {
                value.is_empty()
            };
            *visibility = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            node.display = if show { Display::Flex } else { Display::None };
        }
        if field.0 == Tab::Bindings {
            if searches.bindings != value {
                searches.bindings = value;
                settings.dirty = true;
            }
        } else if field.0 == Tab::Catalogue && searches.catalogue != value {
            searches.catalogue = value;
        }
    }
}
type StyledButtons<'w, 's> = Query<
    'w,
    's,
    (
        &'static Styled,
        &'static bevy::picking::hover::Hovered,
        Has<Quiet>,
        Has<BindingControl>,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
    ),
>;
fn style(mut buttons: StyledButtons) {
    for (style, interaction, quiet, binding, mut background, mut border) in &mut buttons {
        let hover = interaction.0;
        background.0 = if style.primary {
            LEAF
        } else if style.tab || quiet && !hover {
            Color::NONE
        } else if style.selected || hover {
            Color::srgb(0.069, 0.118, 0.081)
        } else if binding {
            BG
        } else {
            CARD
        };
        *border = BorderColor::all(if hover || style.selected || style.primary {
            LEAF
        } else if style.tab || quiet {
            Color::NONE
        } else {
            EDGE
        });
    }
}
fn shell(
    mut commands: Commands,
    (menu, settings): (Res<super::menu::Menu>, Res<Settings>),
    searches: Res<Searches>,
    roots: Query<Entity, With<Shell>>,
    mut previous: Local<Option<Option<(Tab, bool)>>>,
    logo: Res<Mosaic>,
    mut focus: ResMut<InputFocus>,
) {
    let modal = settings.capturing() || settings.modal();
    let wanted = if settings.open {
        Some(settings.tab)
    } else if menu.open {
        Some(Tab::Catalogue)
    } else {
        None
    };
    let wanted = wanted.map(|tab| (tab, modal));
    if *previous == Some(wanted) {
        return;
    }
    let previous_tab = previous
        .as_ref()
        .and_then(|state| state.map(|(tab, _)| tab));
    if previous_tab != wanted.map(|(tab, _)| tab) {
        focus.clear();
    }
    *previous = Some(wanted);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some((tab, _modal)) = wanted else {
        return;
    };
    commands
        .spawn((
            Shell,
            GlobalZIndex(9),
            BackgroundColor(BG),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::axes(Val::Percent(5.0), Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(0.0),
                ..default()
            },
        ))
        .with_children(|root| {
            chrome(root, tab, &logo.0);
            if matches!(tab, Tab::Catalogue | Tab::Bindings) {
                search_row(
                    root,
                    tab,
                    if tab == Tab::Catalogue {
                        &searches.catalogue
                    } else {
                        &searches.bindings
                    },
                );
            }
        });
}
fn chrome(root: &mut ChildSpawnerCommands, tab: Tab, logo: &Handle<Image>) {
    root.spawn((
        Node {
            height: Val::Px(108.0),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            padding: UiRect::bottom(Val::Px(18.0)),
            border: UiRect::bottom(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(EDGE),
    ))
    .with_children(|header| {
        header
            .spawn(Node {
                height: Val::Px(36.0),
                flex_shrink: 0.0,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                row.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    ..default()
                })
                .with_children(|brand| {
                    brand.spawn((
                        ImageNode::new(logo.clone()),
                        Node {
                            width: Val::Px(29.0),
                            height: Val::Px(29.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                    weighted_text(brand, "Tesserae", 17.0, 650, INK);
                });
                compact_button(row, "Back to game", Operation::Back, false, 36.0, 11.0);
            });
        header
            .spawn(Node {
                height: Val::Px(39.0),
                margin: UiRect::top(Val::Px(14.0)),
                column_gap: Val::Px(24.0),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|nav| {
                for page in [Tab::Catalogue, Tab::Behavior, Tab::Bindings, Tab::Display] {
                    button(
                        nav,
                        page.label(),
                        Operation::Navigate(page),
                        page == tab,
                        true,
                        false,
                    );
                }
            });
    });
    root.spawn(Node {
        height: Val::Px(107.0),
        padding: UiRect::vertical(Val::Px(22.0)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: Val::Px(12.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|heading| {
        heading
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|title| {
                weighted_text(title, tab.label(), 25.0, 620, INK);
                text(
                    title,
                    match tab {
                        Tab::Catalogue => "Materials and Shapes for your World.",
                        Tab::Behavior => "Tune how you move, look, and build.",
                        Tab::Bindings => "Find an action and choose up to two bindings.",
                        Tab::Display => "Choose frame pacing without changing visual quality.",
                    },
                    12.0,
                    MUTED,
                );
            });
        heading.spawn((
            HeadingPill(tab),
            Text::default(),
            font(9.0),
            TextColor(MUTED),
            bevy::text::LetterSpacing::Px(0.9),
            BorderColor::all(EDGE),
            Node {
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
                ..default()
            },
            Pickable::IGNORE,
        ));
    });
}
fn search_row(root: &mut ChildSpawnerCommands, tab: Tab, value: &str) {
    root.spawn(Node {
        height: Val::Px(42.0),
        margin: UiRect::bottom(Val::Px(18.0)),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        column_gap: Val::Px(14.0),
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            SearchFrame,
            BackgroundColor(CARD),
            BorderColor::all(EDGE),
            Node {
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                height: Val::Px(42.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                padding: UiRect::axes(Val::Px(11.0), Val::Px(0.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(9.0),
                ..default()
            },
        ))
        .with_children(|field| {
            icon(field, IconKind::Search, MUTED, 14.0);
            field
                .spawn(Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    height: Val::Px(40.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|input| {
                    input.spawn((
                        SearchField(tab),
                        TextInput,
                        EditableText::new(value),
                        TabIndex(0),
                        font(12.0),
                        TextColor(INK),
                        bevy::text::TextCursorStyle {
                            color: LEAF,
                            selection_color: Color::srgba(0.286745, 0.696314, 0.384471, 0.3),
                            ..default()
                        },
                        TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(20.0),
                            ..default()
                        },
                    ));
                    input.spawn((
                        SearchPlaceholder,
                        Text::new(if tab == Tab::Catalogue {
                            "Search materials…"
                        } else {
                            "Search actions or keys…"
                        }),
                        font(12.0),
                        TextColor(MUTED),
                        Pickable::IGNORE,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            ..default()
                        },
                        if value.is_empty() {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        },
                    ));
                });
            let clear = quiet_button(field, "×", ClearSearch, 13.0);
            field.commands().entity(clear).insert(if value.is_empty() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            });
            field.spawn((
                SearchSlash,
                Text::new("/"),
                TextFont {
                    font: bevy::text::FontSource::family("Liberation Mono"),
                    ..font(11.0)
                },
                TextColor(MUTED),
                BorderColor::all(EDGE),
                Node {
                    padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                if value.is_empty() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
                Pickable::IGNORE,
            ));
        });
        row.spawn((
            SearchCount(tab),
            Text::default(),
            font(11.0),
            TextColor(MUTED),
            Node {
                width: Val::Px(72.0),
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            Pickable::IGNORE,
        ));
    });
}

fn make_logo(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    use bevy::{
        asset::RenderAssetUsages,
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    };
    let size = 152;
    let mut data = vec![0; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / 4.0;
            let v = (y as f32 + 0.5) / 4.0;
            let tile = ((2.0..12.0).contains(&u) && (2.0..12.0).contains(&v) && u + v >= 8.0)
                || ((14.0..24.0).contains(&u) && (2.0..12.0).contains(&v))
                || ((26.0..36.0).contains(&u) && (2.0..12.0).contains(&v) && u - v <= 30.0)
                || ((14.0..24.0).contains(&u) && (14.0..24.0).contains(&v))
                || ((14.0..24.0).contains(&u) && (26.0..36.0).contains(&v) && u + v <= 56.0);
            if tile {
                data[(y * size + x) * 4..(y * size + x) * 4 + 4]
                    .copy_from_slice(&[73, 178, 98, 255]);
            }
        }
    }
    commands.insert_resource(Mosaic(images.add(Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))));
}

fn load_fonts(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    commands.insert_resource(Fonts {
        _handles: vec![
            fonts.add(Font::from_bytes(
                include_bytes!("../../../../assets/fonts/LiberationSans-Regular.ttf").to_vec(),
            )),
            fonts.add(Font::from_bytes(
                include_bytes!("../../../../assets/fonts/LiberationSans-Bold.ttf").to_vec(),
            )),
            fonts.add(Font::from_bytes(
                include_bytes!("../../../../assets/fonts/LiberationMono-Regular.ttf").to_vec(),
            )),
        ],
    });
}

pub(super) fn weighted_text(
    parent: &mut ChildSpawnerCommands,
    value: impl Into<String>,
    size: f32,
    weight: u16,
    color: Color,
) {
    parent.spawn((
        Text::new(value),
        TextFont {
            weight: bevy::text::FontWeight(weight),
            ..font(size)
        },
        bevy::text::LineHeight::RelativeToFont(1.5),
        TextColor(color),
        bevy::text::LetterSpacing::Px(if size == 25.0 {
            -0.7
        } else if size == 17.0 {
            -0.5
        } else {
            0.0
        }),
        Pickable::IGNORE,
    ));
}
pub(super) fn compact_button(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: impl Bundle,
    selected: bool,
    height: f32,
    size: f32,
) -> Entity {
    let label = label.into();
    parent
        .spawn((
            Button,
            TabIndex(0),
            operation,
            Styled {
                selected,
                tab: false,
                primary: false,
            },
            Node {
                min_height: Val::Px(height),
                padding: UiRect::axes(Val::Px(11.0), Val::Px(5.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CARD),
            BorderColor::all(if selected { LEAF } else { EDGE }),
        ))
        .with_children(|button| {
            if label == "×" || label == "↺" {
                icon(
                    button,
                    if label == "×" {
                        IconKind::Close
                    } else {
                        IconKind::Reset
                    },
                    MUTED,
                    if label == "×" { 12.0 } else { 16.0 },
                );
            } else {
                text(button, label, size, if selected { LEAF } else { INK });
            }
        })
        .id()
}

fn search_presentation(
    focus: Res<InputFocus>,
    fields: Query<Entity, With<SearchField>>,
    mut frames: Query<&mut BorderColor, With<SearchFrame>>,
    mut counts: Query<(&SearchCount, &mut Text)>,
    searches: Res<Searches>,
    catalogue: Res<super::edit::Catalogue>,
    bindings: (Res<super::actions::Bindings>, Res<super::actions::Labels>),
) {
    let focused = focus.get().is_some_and(|entity| fields.contains(entity));
    for mut border in &mut frames {
        *border = BorderColor::all(if focused { LEAF } else { EDGE });
    }
    for (count, mut text) in &mut counts {
        let (number, label) = if count.0 == Tab::Catalogue {
            (
                catalogue
                    .0
                    .materials
                    .iter()
                    .filter(|m| {
                        (m.solid || m.fluid.is_some()) && matches(&searches.catalogue, &m.name)
                    })
                    .count()
                    + usize::from(
                        catalogue.0.anything_burns() && matches(&searches.catalogue, "fire"),
                    ),
                "materials",
            )
        } else {
            (
                bindings
                    .0
                    .0
                    .keys()
                    .filter(|action| {
                        matches(
                            &searches.bindings,
                            &format!(
                                "{} {} {}",
                                super::settings::action_name(**action),
                                super::settings::category(**action),
                                bindings.0.label(**action, &bindings.1)
                            ),
                        )
                    })
                    .count(),
                "actions",
            )
        };
        text.set_if_neq(Text::new(format!("{number} {label}")));
    }
}

pub(super) fn quiet_button(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: impl Bundle,
    size: f32,
) -> Entity {
    let label = label.into();
    let icon = label == "×" || label == "↺";
    let entity = compact_button(
        parent,
        &label,
        operation,
        false,
        if icon { 25.0 } else { 19.0 },
        size,
    );
    parent.commands().entity(entity).insert((
        Quiet,
        Node {
            width: if label == "×" {
                Val::Px(15.0)
            } else if label == "↺" {
                Val::Px(28.0)
            } else {
                Val::Auto
            },
            min_height: Val::Px(if icon { 25.0 } else { 19.0 }),
            padding: if icon {
                UiRect::ZERO
            } else {
                UiRect::axes(Val::Px(4.0), Val::Px(2.0))
            },
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
    entity
}

#[derive(Clone, Copy)]
pub(super) enum IconKind {
    Search,
    More,
    Reset,
    Close,
    Sync,
    Uncapped,
}
#[derive(Component)]
pub(super) struct Icon(pub IconKind, pub Color);
#[derive(Resource)]
struct IconAtlas(Handle<Image>);
fn make_icons(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    use bevy::{
        asset::RenderAssetUsages,
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    };
    let mut data = vec![0; 384 * 64 * 4];
    let segment = |p: Vec2, a: Vec2, b: Vec2| {
        let along = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
        p.distance(a + (b - a) * along) < 0.6
    };
    for y in 0..64 {
        for x in 0..384 {
            let i = x / 64;
            let p = Vec2::new(((x % 64) as f32 + 0.5) / 4.0, (y as f32 + 0.5) / 4.0);
            let ink = match i {
                0 => {
                    (p.distance(Vec2::new(6.5, 6.5)) - 3.5).abs() < 0.6
                        || segment(p, Vec2::new(9.0, 9.0), Vec2::new(13.0, 13.0))
                }
                1 => [4.0, 8.0, 12.0]
                    .iter()
                    .any(|u| p.distance(Vec2::new(*u, 8.0)) < 0.8),
                2 => {
                    let d = p - Vec2::splat(8.0);
                    (d.length() - 4.8).abs() < 0.6 && !(d.x < 0.0 && d.y < 0.0)
                        || segment(p, Vec2::new(3.0, 3.0), Vec2::new(3.0, 7.0))
                        || segment(p, Vec2::new(3.0, 3.0), Vec2::new(7.0, 3.0))
                }
                3 => {
                    segment(p, Vec2::splat(5.0), Vec2::splat(11.0))
                        || segment(p, Vec2::new(5.0, 11.0), Vec2::new(11.0, 5.0))
                }
                4 => {
                    segment(p, Vec2::new(2.0, 8.0), Vec2::new(14.0, 8.0))
                        || segment(p, Vec2::new(2.0, 8.0), Vec2::new(5.0, 5.0))
                        || segment(p, Vec2::new(2.0, 8.0), Vec2::new(5.0, 11.0))
                        || segment(p, Vec2::new(14.0, 8.0), Vec2::new(11.0, 5.0))
                        || segment(p, Vec2::new(14.0, 8.0), Vec2::new(11.0, 11.0))
                }
                _ => {
                    segment(p, Vec2::new(4.0, 12.0), Vec2::new(12.0, 4.0))
                        || segment(p, Vec2::new(6.0, 4.0), Vec2::new(12.0, 4.0))
                        || segment(p, Vec2::new(12.0, 10.0), Vec2::new(12.0, 4.0))
                }
            };
            if ink {
                data[(y * 384 + x) * 4..(y * 384 + x) * 4 + 4].copy_from_slice(&[255; 4]);
            }
        }
    }
    commands.insert_resource(IconAtlas(images.add(Image::new(
        Extent3d {
            width: 384,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))));
}
fn materialize_icons(
    mut commands: Commands,
    atlas: Res<IconAtlas>,
    icons: Query<(Entity, &Icon), Added<Icon>>,
) {
    for (entity, icon) in &icons {
        let index = icon.0 as u32;
        commands.entity(entity).insert(ImageNode {
            image: atlas.0.clone(),
            rect: Some(Rect::new(
                (index * 64) as f32,
                0.0,
                ((index + 1) * 64) as f32,
                64.0,
            )),
            color: icon.1,
            ..default()
        });
    }
}
pub(super) fn icon(parent: &mut ChildSpawnerCommands, kind: IconKind, color: Color, size: f32) {
    parent.spawn((
        Icon(kind, color),
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ));
}

pub(super) fn eyebrow(parent: &mut ChildSpawnerCommands, label: &str) {
    parent.spawn((
        Text::new(label),
        font(9.0),
        bevy::text::LineHeight::RelativeToFont(1.5),
        bevy::text::LetterSpacing::Px(1.2),
        TextColor(MUTED),
        Pickable::IGNORE,
    ));
}
pub(super) fn mono(parent: &mut ChildSpawnerCommands, label: &str, size: f32, color: Color) {
    parent.spawn((
        Text::new(label),
        TextFont {
            font: bevy::text::FontSource::family("Liberation Mono"),
            ..font(size)
        },
        bevy::text::LineHeight::RelativeToFont(1.5),
        TextColor(color),
        Pickable::IGNORE,
    ));
}
fn heading_status(
    mut pills: Query<(&HeadingPill, &mut Text)>,
    settings: Res<Settings>,
    active: (
        Res<super::actions::Bindings>,
        Res<super::profile::Preferences>,
    ),
    catalogue: Res<super::edit::Catalogue>,
) {
    for (pill, mut text) in &mut pills {
        let value = if pill.0 == Tab::Catalogue {
            format!(
                "{} MATERIALS",
                catalogue.0.materials.len() + usize::from(catalogue.0.anything_burns())
            )
        } else if settings.changed(&active.0, &active.1) {
            "UNSAVED CHANGES".into()
        } else {
            "SAVED PREFERENCES".into()
        };
        text.set_if_neq(Text::new(value));
    }
}
type ButtonLabels<'w, 's> = Query<
    'w,
    's,
    (
        &'static Styled,
        &'static bevy::picking::hover::Hovered,
        &'static Children,
        &'static ChildOf,
        Has<bevy::ui::InteractionDisabled>,
        Has<Quiet>,
    ),
>;
fn button_label_colors(
    buttons: ButtonLabels,
    segments: Query<(), With<Segment>>,
    mut labels: Query<&mut TextColor>,
    mut images: Query<&mut ImageNode>,
) {
    for (style, hovered, children, parent, disabled, quiet) in &buttons {
        let color = if disabled {
            MUTED.with_alpha(0.55)
        } else if style.primary {
            BG
        } else if (quiet || style.tab && !style.selected) && !hovered.0 {
            MUTED
        } else if style.selected && !style.tab && !segments.contains(parent.parent()) {
            LEAF
        } else {
            INK
        };
        for child in children {
            if let Ok(mut text) = labels.get_mut(*child) {
                text.0 = color;
            }
            if let Ok(mut image) = images.get_mut(*child) {
                image.color = color;
            }
        }
    }
}

#[derive(Component)]
struct Notification(Timer);
pub(super) fn notify(commands: &mut Commands, message: &str) {
    commands
        .spawn((
            Notification(Timer::from_seconds(1.8, TimerMode::Once)),
            GlobalZIndex(40),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(5.0),
                right: Val::Percent(5.0),
                top: Val::Px(89.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            root.spawn((
                BackgroundColor(CARD),
                BorderColor::all(EDGE),
                Node {
                    padding: UiRect::axes(Val::Px(15.0), Val::Px(9.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|toast| text(toast, message, 12.0, INK));
        });
}
fn age_notifications(
    mut commands: Commands,
    time: Option<Res<Time>>,
    mut notifications: Query<(Entity, &mut Notification)>,
) {
    let Some(time) = time else {
        return;
    };
    for (entity, mut notice) in &mut notifications {
        if notice.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub(super) fn description(parent: &mut ChildSpawnerCommands, value: &str) {
    parent.spawn((
        Text::new(value),
        font(11.0),
        bevy::text::LineHeight::RelativeToFont(1.65),
        TextColor(MUTED),
        Pickable::IGNORE,
    ));
}
pub(super) fn paragraph(
    parent: &mut ChildSpawnerCommands,
    value: impl Into<String>,
    size: f32,
    color: Color,
) {
    parent.spawn((
        Text::new(value),
        font(size),
        bevy::text::LineHeight::RelativeToFont(1.6),
        TextColor(color),
        Pickable::IGNORE,
    ));
}
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .init_resource::<Settings>()
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<Searches>()
            .init_resource::<InputFocus>()
            .init_resource::<super::super::input::UiInput>()
            .init_resource::<Assets<Image>>()
            .add_systems(Startup, make_logo)
            .add_systems(Update, (sync_search, shell).chain());
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app
    }
    fn field(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, With<SearchField>>()
            .single(app.world())
            .unwrap()
    }
    #[test]
    fn filtering_preserves_the_native_editor_and_focus_and_retains_each_query() {
        let mut app = app();
        app.update();
        let editor = field(&mut app);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(editor, FocusCause::Pressed);
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .unwrap()
            .editor
            .set_text("Stone");
        app.update();
        assert_eq!(field(&mut app), editor);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
        assert_eq!(app.world().resource::<Searches>().catalogue, "Stone");
        {
            let mut settings = app.world_mut().resource_mut::<Settings>();
            settings.open = true;
            settings.tab = Tab::Bindings;
        }
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        app.update();
        assert!(app.world().resource::<Searches>().bindings.is_empty());
        let bindings = field(&mut app);
        app.world_mut()
            .get_mut::<EditableText>(bindings)
            .unwrap()
            .editor
            .set_text("mouse");
        app.update();
        assert_eq!(app.world().resource::<Searches>().bindings, "mouse");
        app.world_mut().resource_mut::<Settings>().tab = Tab::Display;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&SearchField>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<Settings>().open = false;
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.update();
        let restored = field(&mut app);
        assert_eq!(
            app.world()
                .get::<EditableText>(restored)
                .unwrap()
                .value()
                .to_string(),
            "Stone"
        );
    }
    #[test]
    fn escape_from_search_consumes_cancellation_even_when_the_widget_releases_focus() {
        let mut app = app();
        app.update();
        let editor = field(&mut app);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(editor, FocusCause::Pressed);
        app.world_mut().run_system_once(focus).unwrap();
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .unwrap()
            .editor
            .set_text("i 5 tab");
        app.world_mut().resource_mut::<InputFocus>().clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.world_mut().run_schedule(PreUpdate);
        app.world_mut().run_system_once(focus).unwrap();
        app.world_mut().run_system_once(search_keys).unwrap();
        assert!(
            !app.world()
                .resource::<super::super::actions::Actions>()
                .cancel_requested()
        );
        assert!(
            app.world()
                .resource::<super::super::input::UiInput>()
                .text_entry
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
        assert!(
            matches!(app.world().get::<EditableText>(editor).unwrap().pending_edits.last(),Some(TextEdit::Insert(text)) if text.is_empty())
        );
    }
}
