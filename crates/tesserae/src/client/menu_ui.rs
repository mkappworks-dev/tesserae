//! Shared menu chrome; page bodies never replace the focused search widget.
use super::settings::{Operation, Settings};
use bevy::{
    input_focus::{FocusCause, InputFocus, InputFocusSystems, tab_navigation::TabIndex},
    prelude::*,
    text::{EditableText, TextCursorStyle, TextEdit},
    ui_widgets::{Activate, Button, TextInput},
};

pub(super) const LEAF: Color = Color::srgb(0.27, 0.70, 0.37);
pub(super) const BG: Color = Color::srgb(0.035, 0.04, 0.035);
pub(super) const CARD: Color = Color::srgb(0.065, 0.075, 0.065);
pub(super) const EDGE: Color = Color::srgb(0.18, 0.21, 0.18);
pub(super) const INK: Color = Color::srgb(0.92, 0.94, 0.91);
pub(super) const MUTED: Color = Color::srgb(0.57, 0.62, 0.57);

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
#[derive(Component)]
pub(super) struct SearchField;
#[derive(Component)]
struct ClearSearch;
#[derive(Component)]
struct SearchPlaceholder;
#[derive(Component)]
pub(super) struct Styled {
    pub selected: bool,
    pub tab: bool,
    pub primary: bool,
}

pub(super) fn font(size: f32) -> TextFont {
    TextFont {
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
                min_height: Val::Px(36.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
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
            font(13.0),
            TextColor(if primary { BG } else { INK }),
            Pickable::IGNORE,
        ))
        .id()
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Searches>()
        .init_resource::<InputFocus>()
        .add_observer(clear_search)
        .add_systems(Startup, make_logo)
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
        || (ui.text_entry && keys.just_pressed(KeyCode::Escape));
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
fn sync_search(
    fields: Query<&EditableText, With<SearchField>>,
    mut searches: ResMut<Searches>,
    mut settings: ResMut<Settings>,
    mut placeholders: Query<&mut Visibility, With<SearchPlaceholder>>,
) {
    if let Some(text) = fields.iter().next() {
        let value = text.value().to_string();
        for mut visibility in &mut placeholders {
            *visibility = if value.is_empty() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if settings.open && settings.tab == Tab::Bindings {
            if searches.bindings != value {
                searches.bindings = value;
                settings.dirty = true;
            }
        } else if !settings.open && searches.catalogue != value {
            searches.catalogue = value;
        }
    }
}
fn style(
    mut buttons: Query<(
        &Styled,
        &bevy::picking::hover::Hovered,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (style, interaction, mut background, mut border) in &mut buttons {
        let hover = interaction.0;
        background.0 = if style.primary {
            LEAF
        } else if style.tab {
            Color::NONE
        } else if style.selected || hover {
            Color::srgb(0.10, 0.16, 0.10)
        } else {
            CARD
        };
        *border = BorderColor::all(if hover || style.selected || style.primary {
            LEAF
        } else if style.tab {
            Color::NONE
        } else {
            EDGE
        });
    }
}
fn shell(
    mut commands: Commands,
    menu: Res<super::menu::Menu>,
    settings: Res<Settings>,
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
    *previous = Some(wanted);
    focus.clear();
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some((tab, modal)) = wanted else {
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
                row_gap: Val::Px(14.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
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
                        ImageNode::new(logo.0.clone()),
                        Node {
                            width: Val::Px(34.0),
                            height: Val::Px(34.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                    text(brand, "Tesserae", 21.0, INK);
                });
                row.spawn(Node {
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|actions| {
                    button(
                        actions,
                        "Back to game",
                        Operation::Back,
                        false,
                        false,
                        false,
                    );
                    super::save::quit_button(actions);
                });
            });
            root.spawn(Node {
                column_gap: Val::Px(12.0),
                border: UiRect::bottom(Val::Px(1.0)),
                padding: UiRect::bottom(Val::Px(5.0)),
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
            text(root, tab.label(), 25.0, INK);
            text(
                root,
                match tab {
                    Tab::Catalogue => "Materials and Shapes for your World.",
                    Tab::Behavior => "Tune how you move, look, and build.",
                    Tab::Bindings => "Choose up to two bindings per action.",
                    Tab::Display => "Choose frame pacing without changing visual quality.",
                },
                12.0,
                MUTED,
            );
            if !modal && matches!(tab, Tab::Catalogue | Tab::Bindings) {
                let value = if tab == Tab::Catalogue {
                    &searches.catalogue
                } else {
                    &searches.bindings
                };
                root.spawn(Node {
                    height: Val::Px(40.0),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        BackgroundColor(CARD),
                        BorderColor::all(EDGE),
                        Node {
                            flex_grow: 1.0,
                            height: Val::Px(40.0),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(5.0)),
                            padding: UiRect::all(Val::Px(10.0)),
                            ..default()
                        },
                    ))
                    .with_children(|field| {
                        field.spawn((
                            SearchField,
                            TextInput,
                            EditableText::new(value),
                            TabIndex(0),
                            font(14.0),
                            TextColor(INK),
                            TextCursorStyle {
                                color: LEAF,
                                selection_color: Color::srgb(0.14, 0.32, 0.17),
                                ..default()
                            },
                            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(22.0),
                                ..default()
                            },
                        ));
                        field.spawn((
                            SearchPlaceholder,
                            Text::new(if tab == Tab::Catalogue {
                                "Search materials..."
                            } else {
                                "Search actions or keys..."
                            }),
                            font(13.0),
                            TextColor(MUTED),
                            Pickable::IGNORE,
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(10.0),
                                top: Val::Px(10.0),
                                ..default()
                            },
                            if value.is_empty() {
                                Visibility::Inherited
                            } else {
                                Visibility::Hidden
                            },
                        ));
                    });
                    button(row, "Clear", ClearSearch, false, false, false);
                });
            }
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
                    .copy_from_slice(&[69, 179, 94, 255]);
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
