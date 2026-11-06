//! Shared menu chrome; page bodies never replace the focused search widget.
use super::settings::{Operation, Settings};
use crate::client::buttons::{self, ButtonStyle, Eyebrow};
use crate::session::SessionApp;
use bevy::{
    input_focus::{FocusCause, InputFocus, InputFocusSystems, tab_navigation::TabIndex},
    prelude::*,
    text::{EditableText, TextEdit},
    ui_widgets::{Activate, TextInput},
};

use super::theme;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(super) enum Tab {
    Catalogue,
    #[default]
    Behavior,
    Bindings,
    Controller,
    Display,
    Accessibility,
}
impl Tab {
    /// Page order in the header, which LB/RB also follow.
    pub const ORDER: [Tab; 5] = [
        Tab::Behavior,
        Tab::Bindings,
        Tab::Controller,
        Tab::Display,
        Tab::Accessibility,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Catalogue => "Catalogue",
            Self::Behavior => "Behavior",
            Self::Bindings => "Key bindings",
            Self::Controller => "Controller",
            Self::Display => "Display",
            Self::Accessibility => "Accessibility",
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
/// A declared name as a player reads it: `terra:ash_flats` is "Ash flats".
pub(super) fn declared_title(name: &str) -> String {
    title(&name.rsplit(':').next().unwrap_or(name).replace('_', " "))
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
pub(super) struct Mosaic(pub(super) Handle<Image>);
#[derive(Resource)]
struct Fonts {
    _handles: Vec<Handle<Font>>,
}
#[derive(Component)]
pub(super) struct SearchField(Tab);
#[derive(Component)]
struct ClearSearch;
#[derive(Component)]
pub(super) struct Quiet;
#[derive(Component)]
pub(super) struct EmptyBinding;
#[derive(Component)]
pub(super) struct EmptyBorder(pub Entity);
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
#[require(bevy::picking::hover::Hovered, Outline = theme::focus_ring())]
pub(super) struct Styled {
    pub selected: bool,
    pub tab: bool,
    pub primary: bool,
}

/// The focused entity when focus is being shown, as after pad navigation.
pub(super) fn visible_focus(
    focus: Option<&InputFocus>,
    visible: Option<&bevy::input_focus::InputFocusVisible>,
) -> Option<Entity> {
    visible
        .is_some_and(|visible| visible.0)
        .then(|| focus.and_then(InputFocus::get))
        .flatten()
}

pub(super) fn font(size: f32) -> TextFont {
    theme::font(size)
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

pub(super) fn plugin(app: &mut App) {
    // The chrome the World list shares with the in-World menu.
    app.init_resource::<InputFocus>()
        .add_systems(Startup, (make_logo, load_fonts, make_icons))
        .add_systems(
            PostUpdate,
            materialize_icons.before(bevy::ui::UiSystems::Layout),
        )
        .add_systems(
            Update,
            (style, button_label_colors)
                .chain()
                .after(super::settings::render),
        )
        .add_systems(Update, age_notifications.after(button_label_colors));
    use crate::session::in_menus;
    app.init_session_state::<Searches>()
        .add_observer(clear_search)
        .add_systems(
            Update,
            (search_presentation, heading_status)
                .after(shell)
                .run_if(in_menus),
        )
        .add_systems(
            PreUpdate,
            focus
                .after(InputFocusSystems::Dispatch)
                .after(bevy::picking::PickingSystems::Last)
                .run_if(in_menus),
        )
        .add_systems(
            Update,
            dashed_border_colors
                .after(button_label_colors)
                .run_if(in_menus),
        )
        .add_systems(
            Update,
            (search_keys, sync_search)
                .chain()
                .before(super::menu::toggle)
                .before(super::settings::capture)
                .run_if(in_menus),
        )
        .add_systems(
            Update,
            shell
                .before(style)
                .before(super::settings::render)
                .run_if(in_menus),
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
        Entity,
        &'static Styled,
        &'static bevy::picking::hover::Hovered,
        Has<Quiet>,
        Has<EmptyBinding>,
        Has<bevy::ui::InteractionDisabled>,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
        &'static mut Outline,
    ),
>;
fn style(
    mut buttons: StyledButtons,
    focus: Res<InputFocus>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let focused = visible_focus(Some(&focus), visible.as_deref());
    for (
        entity,
        style,
        interaction,
        quiet,
        empty,
        disabled,
        mut background,
        mut border,
        mut outline,
    ) in &mut buttons
    {
        let hover = interaction.0 && !disabled;
        let (fill, edge) = if empty || style.tab {
            (
                Color::NONE,
                if style.tab && style.selected {
                    theme::SAGE
                } else {
                    Color::NONE
                },
            )
        } else if disabled {
            (theme::DISABLED, theme::DISABLED_EDGE)
        } else if style.primary {
            (
                if hover {
                    theme::SAGE_HOVER
                } else {
                    theme::SAGE
                },
                theme::SAGE,
            )
        } else if quiet {
            if hover {
                (theme::QUIET_HOVER, theme::QUIET_HOVER_EDGE)
            } else {
                (Color::NONE, Color::NONE)
            }
        } else if style.selected {
            (theme::SEGMENT_ON, theme::SEGMENT_ON_EDGE)
        } else if hover {
            (theme::HOVER, theme::EDGE)
        } else {
            (theme::RAISED, theme::EDGE)
        };
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
    }
}
/// Where the Settings page's body goes, under the sheet's header.
#[derive(Component)]
pub(super) struct SettingsBody;

/// The Settings frame: the category rail beside a sheet whose body the page
/// fills. It is rebuilt only when the category or a dialog changes, so the
/// focused search field survives a page rebuild.
#[allow(clippy::too_many_arguments)]
fn shell(
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    searches: Res<Searches>,
    preferences: Res<super::profile::Preferences>,
    roots: Query<Entity, With<Shell>>,
    mut previous: Local<Option<Option<(Tab, bool, bool)>>>,
    mut focus: ResMut<InputFocus>,
    playing: Option<Res<State<crate::session::AppState>>>,
) {
    use super::cinematic as cine;
    let modal = settings.capturing() || settings.modal();
    let wanted = settings
        .open
        .then_some((settings.tab, modal, preferences.solid_background));
    if *previous == Some(wanted) {
        return;
    }
    let previous_tab = previous
        .as_ref()
        .and_then(|state| state.map(|(tab, ..)| tab));
    if previous_tab != wanted.map(|(tab, ..)| tab) {
        focus.clear();
    }
    *previous = Some(wanted);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some((tab, _modal, solid)) = wanted else {
        return;
    };
    // The page lands in the new body this frame.
    settings.dirty = true;
    let over_world = playing.is_some_and(|s| *s.get() == crate::session::AppState::InWorld);
    commands
        .spawn((
            Shell,
            GlobalZIndex(9),
            bevy::input_focus::tab_navigation::TabGroup {
                modal: true,
                ..default()
            },
            BackgroundColor(if over_world && !solid {
                Color::NONE
            } else if solid {
                theme::SOLID_BACKGROUND
            } else {
                theme::SCENE
            }),
            if over_world && !solid {
                theme::shade()
            } else {
                BackgroundGradient::default()
            },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(theme::SCREEN_PAD_X), Val::Px(28.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(Node {
                flex_grow: 1.0,
                max_width: Val::Px(1180.0),
                min_height: Val::Px(0.0),
                column_gap: Val::Px(24.0),
                ..default()
            })
            .with_children(|workspace| {
                rail(workspace, tab);
                workspace
                    .spawn((
                        cine::surface(theme::PANEL),
                        cine::framed(Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            min_width: Val::Px(0.0),
                            min_height: Val::Px(0.0),
                            overflow: Overflow::clip(),
                            ..default()
                        }),
                    ))
                    .with_children(|sheet| {
                        heading(sheet, tab);
                        if tab == Tab::Bindings {
                            sheet
                                .spawn(Node {
                                    padding: UiRect::new(
                                        Val::Px(26.0),
                                        Val::Px(26.0),
                                        Val::Px(16.0),
                                        Val::Px(0.0),
                                    ),
                                    flex_shrink: 0.0,
                                    ..default()
                                })
                                .with_children(|row| search_row(row, tab, &searches.bindings));
                        }
                        sheet.spawn((
                            SettingsBody,
                            Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                ..default()
                            },
                        ));
                    });
            });
        });
}

fn rail(workspace: &mut ChildSpawnerCommands, tab: Tab) {
    use super::cinematic as cine;
    workspace
        .spawn((
            cine::surface(theme::PANEL),
            cine::framed(Node {
                width: Val::Px(220.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(7.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(20.0)),
                ..default()
            }),
        ))
        .with_children(|nav| {
            nav.spawn(Node {
                padding: UiRect::new(Val::Px(9.0), Val::Px(0.0), Val::Px(0.0), Val::Px(10.0)),
                ..default()
            })
            .with_children(|title| buttons::eyebrow(title, "SETTINGS", Eyebrow::Cinematic));
            for page in Tab::ORDER {
                let entry = buttons::button(
                    nav,
                    page.label(),
                    Operation::Navigate(page),
                    ButtonStyle::Choice {
                        selected: page == tab,
                    },
                );
                nav.commands().entity(entry).insert(Node {
                    min_height: Val::Px(44.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                });
            }
            nav.spawn(Node {
                margin: UiRect::top(Val::Auto),
                padding: UiRect::horizontal(Val::Px(9.0)),
                ..default()
            })
            .with_children(|note| {
                paragraph(
                    note,
                    "Your preferences, across every World.",
                    theme::TEXT_EYEBROW,
                    theme::CHROME_INK,
                )
            });
        });
}

fn heading(sheet: &mut ChildSpawnerCommands, tab: Tab) {
    use super::cinematic as cine;
    sheet
        .spawn((
            BackgroundColor(theme::SHEET_HEADER),
            BorderColor::all(theme::EDGE),
            Node {
                padding: UiRect::new(
                    Val::Px(theme::SHEET_PAD_X),
                    Val::Px(theme::SHEET_PAD_X),
                    Val::Px(20.0),
                    Val::Px(18.0),
                ),
                border: UiRect::bottom(Val::Px(theme::RULE)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|header| {
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(7.0),
                    ..default()
                })
                .with_children(|title| {
                    buttons::eyebrow(title, "MENU / SETTINGS", Eyebrow::Cinematic);
                    cine::title(title, tab.label());
                    paragraph(
                        title,
                        match tab {
                            Tab::Catalogue | Tab::Behavior => "Tune how you move, look, and build.",
                            Tab::Bindings => "Find an action and choose up to two bindings.",
                            Tab::Controller => {
                                "Tune the sticks and choose controller buttons, and the touch controls."
                            }
                            Tab::Display => "Choose frame pacing without changing visual quality.",
                            Tab::Accessibility => "Comfort and readability, in menus and in play.",
                        },
                        theme::TEXT_SMALL,
                        theme::MUTED,
                    );
                });
            header.spawn((
                HeadingPill(tab),
                Text::default(),
                font(10.0),
                TextColor(theme::MUTED),
                bevy::text::LetterSpacing::Px(0.9),
                BorderColor::all(theme::EDGE),
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

pub(super) fn search_frame(
    parent: &mut ChildSpawnerCommands,
    tab: Tab,
    value: &str,
    height: f32,
) -> Entity {
    let mut id = Entity::PLACEHOLDER;
    parent
        .spawn((
            SearchFrame,
            BackgroundColor(theme::FIELD),
            BorderColor::all(theme::FIELD_EDGE),
            Node {
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                height: Val::Px(height),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                padding: UiRect::axes(Val::Px(11.0), Val::Px(0.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(9.0),
                ..default()
            },
        ))
        .with_children(|field| {
            icon(field, IconKind::Search, theme::MUTED, 16.0);
            field
                .spawn(Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    height: Val::Px(height - 2.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|input| {
                    id = input
                        .spawn((
                            SearchField(tab),
                            TextInput,
                            EditableText::new(value),
                            TabIndex(0),
                            font(theme::TEXT_SMALL),
                            TextColor(theme::INK),
                            bevy::text::TextCursorStyle {
                                color: theme::SAGE,
                                selection_color: theme::SAGE.with_alpha(0.3),
                                ..default()
                            },
                            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(20.0),
                                ..default()
                            },
                        ))
                        .id();
                    input.spawn((
                        SearchPlaceholder,
                        Text::new(if tab == Tab::Catalogue {
                            "Search materials…"
                        } else {
                            "Search actions or keys…"
                        }),
                        font(theme::TEXT_SMALL),
                        TextColor(theme::PLACEHOLDER),
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
            let clear = buttons::button(
                field,
                "×",
                ClearSearch,
                ButtonStyle::Quiet { text_px: 13.0 },
            );
            field.commands().entity(clear).insert(if value.is_empty() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            });
            field.spawn((
                SearchSlash,
                Text::new("/"),
                font(theme::TEXT_KEY),
                TextColor(theme::MUTED),
                BorderColor::all(theme::KEY_EDGE),
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
    id
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
        search_frame(row, tab, value, 42.0);
        row.spawn((
            SearchCount(tab),
            Text::default(),
            font(theme::TEXT_EYEBROW),
            TextColor(theme::MUTED),
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
    // The five tiles of the mosaic T on a 38-unit grid, as `brand.svg` draws them.
    let tile = |u: f32, v: f32| -> Option<usize> {
        if (2.0..12.0).contains(&u) && (2.0..12.0).contains(&v) && u + v >= 8.0 {
            Some(0)
        } else if (14.0..24.0).contains(&u) && (2.0..12.0).contains(&v) {
            Some(1)
        } else if (26.0..36.0).contains(&u) && (2.0..12.0).contains(&v) && u - v <= 30.0 {
            Some(2)
        } else if (14.0..24.0).contains(&u) && (14.0..24.0).contains(&v) {
            Some(3)
        } else if (14.0..24.0).contains(&u) && (26.0..36.0).contains(&v) && u + v <= 56.0 {
            Some(4)
        } else {
            None
        }
    };
    const SAGE: [[u8; 4]; 5] = [
        [168, 183, 156, 255],
        [219, 228, 208, 255],
        [168, 183, 156, 255],
        [193, 205, 181, 255],
        [147, 164, 135, 255],
    ];
    const GROUT: [u8; 4] = [22, 27, 22, 255];
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / 4.0;
            let v = (y as f32 + 0.5) / 4.0;
            let Some(index) = tile(u, v) else {
                continue;
            };
            let edge = [(-0.5, 0.0), (0.5, 0.0), (0.0, -0.5), (0.0, 0.5)]
                .iter()
                .any(|(du, dv)| tile(u + du, v + dv) != Some(index));
            data[(y * size + x) * 4..(y * size + x) * 4 + 4].copy_from_slice(if edge {
                &GROUT
            } else {
                &SAGE[index]
            });
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
    // The body face replaces the default asset, so HUD and save dialogs use it too.
    fonts
        .insert(
            bevy::asset::AssetId::default(),
            Font::from_bytes(include_bytes!("../../../../assets/fonts/InterVariable.ttf").to_vec()),
        )
        .expect("the default Font asset can be replaced");
    commands.insert_resource(Fonts {
        _handles: vec![fonts.add(Font::from_bytes(
            include_bytes!("../../../../assets/fonts/Aileron-SemiBold.otf").to_vec(),
        ))],
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
        theme::weighted(size, weight),
        bevy::text::LineHeight::RelativeToFont(1.5),
        TextColor(color),
        Pickable::IGNORE,
    ));
}

fn search_presentation(
    focus: Res<InputFocus>,
    fields: Query<Entity, With<SearchField>>,
    mut frames: Query<&mut BorderColor, With<SearchFrame>>,
    mut counts: Query<(&SearchCount, &mut Text)>,
    searches: Res<Searches>,
    (catalogue, settings): (Res<super::edit::Catalogue>, Res<Settings>),
    bindings: (Res<super::actions::Bindings>, Res<super::actions::Labels>),
) {
    let focused = focus.get().is_some_and(|entity| fields.contains(entity));
    for mut border in &mut frames {
        *border = BorderColor::all(if focused {
            theme::SAGE
        } else {
            theme::FIELD_EDGE
        });
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
                    .keyboard
                    .keys()
                    .filter(|action| {
                        settings.binding_visible(
                            **action,
                            &searches.bindings,
                            &bindings.0,
                            &bindings.1,
                        )
                    })
                    .count(),
                "actions",
            )
        };
        let label = if number == 1 {
            if label == "materials" {
                "material"
            } else {
                "action"
            }
        } else {
            label
        };
        text.set_if_neq(Text::new(format!("{number} {label}")));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IconKind {
    Search,
    Reset,
    Close,
    Sync,
    Uncapped,
    /// Inter has no ⚙.
    Settings,
    Mods,
    Check,
}
const ICONS: usize = 8;
/// The icons drawn from the prototype's 24-unit outlines, in atlas order.
const DRAWN: [IconKind; 3] = [IconKind::Settings, IconKind::Mods, IconKind::Check];
#[derive(Component)]
pub(super) struct Icon(pub IconKind, pub Color);
#[derive(Resource)]
struct IconAtlas {
    icons: Handle<Image>,
    empty_border: Handle<Image>,
}
fn make_icons(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    use bevy::{
        asset::RenderAssetUsages,
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    };
    let width = ICONS * 64;
    let mut data = vec![0; width * 64 * 4];
    let segment = |p: Vec2, a: Vec2, b: Vec2| {
        let along = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
        p.distance(a + (b - a) * along) < 0.6
    };
    for y in 0..64 {
        for x in 0..width {
            let i = x / 64;
            let p = Vec2::new(((x % 64) as f32 + 0.5) / 4.0, (y as f32 + 0.5) / 4.0);
            if let Some(kind) = DRAWN.iter().find(|k| **k as usize == i) {
                let p = Vec2::new((x % 64) as f32 + 0.5, y as f32 + 0.5) * 24.0 / 64.0;
                let alpha = (drawn(*kind, p) * 255.0).round() as u8;
                data[(y * width + x) * 4..(y * width + x) * 4 + 4]
                    .copy_from_slice(&[255, 255, 255, alpha]);
                continue;
            }
            let ink = match i {
                0 => {
                    (p.distance(Vec2::new(6.5, 6.5)) - 3.5).abs() < 0.6
                        || segment(p, Vec2::new(9.0, 9.0), Vec2::new(13.0, 13.0))
                }
                1 => {
                    let d = p - Vec2::splat(8.0);
                    (d.length() - 4.8).abs() < 0.6 && !(d.x < 0.0 && d.y < 0.0)
                        || segment(p, Vec2::new(3.0, 3.0), Vec2::new(3.0, 7.0))
                        || segment(p, Vec2::new(3.0, 3.0), Vec2::new(7.0, 3.0))
                }
                2 => {
                    segment(p, Vec2::splat(5.0), Vec2::splat(11.0))
                        || segment(p, Vec2::new(5.0, 11.0), Vec2::new(11.0, 5.0))
                }
                3 => {
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
                data[(y * width + x) * 4..(y * width + x) * 4 + 4].copy_from_slice(&[255; 4]);
            }
        }
    }
    let icons = images.add(Image::new(
        Extent3d {
            width: width as u32,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let mut border = vec![0; 150 * 33 * 4];
    for y in 0..33 {
        for x in 0..150 {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let q = (p - Vec2::new(75.0, 16.5)).abs() - Vec2::new(71.0, 12.5);
            let distance = q.max(Vec2::ZERO).length() + q.max_element().min(0.0) - 4.0;
            let along = if !(4..146).contains(&x) { y } else { x };
            if (-1.0..=0.0).contains(&distance) && along % 7 < 4 {
                border[(y * 150 + x) * 4..(y * 150 + x) * 4 + 4].copy_from_slice(&[255; 4]);
            }
        }
    }
    let empty_border = images.add(Image::new(
        Extent3d {
            width: 150,
            height: 33,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        border,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    commands.insert_resource(IconAtlas {
        icons,
        empty_border,
    });
}
/// How much of a pixel at `p`, in the prototype's 24-unit icon grid, an
/// icon covers; strokes are its 1.7-unit round-capped lines.
fn drawn(kind: IconKind, p: Vec2) -> f32 {
    const UNITS_PER_PIXEL: f32 = 24.0 / 64.0;
    let v = Vec2::new;
    let segment = |a: Vec2, b: Vec2| {
        let along = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
        p.distance(a + (b - a) * along)
    };
    let path = |points: &[Vec2]| {
        points
            .windows(2)
            .map(|w| segment(w[0], w[1]))
            .fold(f32::MAX, f32::min)
    };
    let stroke = |distance: f32| ((0.85 - distance) / UNITS_PER_PIXEL + 0.5).clamp(0.0, 1.0);
    let fill = |signed: f32| (0.5 - signed / UNITS_PER_PIXEL).clamp(0.0, 1.0);
    match kind {
        IconKind::Mods => stroke(
            path(&[
                v(12.0, 3.0),
                v(21.0, 8.0),
                v(12.0, 13.0),
                v(3.0, 8.0),
                v(12.0, 3.0),
            ])
            .min(path(&[
                v(3.0, 8.0),
                v(3.0, 17.0),
                v(12.0, 22.0),
                v(21.0, 17.0),
                v(21.0, 8.0),
            ]))
            .min(segment(v(12.0, 13.0), v(12.0, 22.0))),
        ),
        IconKind::Check => stroke(path(&[v(5.0, 12.0), v(9.0, 16.0), v(19.0, 6.0)])),
        _ => {
            let d = p - v(12.0, 12.0);
            let teeth = (d.y.atan2(d.x) * 8.0).cos();
            let outer = if teeth > 0.0 { 9.5 } else { 7.6 };
            fill((d.length() - outer).max(3.2 - d.length()))
        }
    }
}
fn materialize_icons(
    mut commands: Commands,
    atlas: Res<IconAtlas>,
    icons: Query<(Entity, &Icon), Added<Icon>>,
    borders: Query<Entity, Added<EmptyBorder>>,
) {
    for (entity, icon) in &icons {
        let index = icon.0 as u32;
        commands.entity(entity).insert(ImageNode {
            image: atlas.icons.clone(),
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
    for entity in &borders {
        commands.entity(entity).insert(ImageNode {
            image: atlas.empty_border.clone(),
            color: theme::EDGE,
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

pub(super) fn technical_text(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    size: f32,
    color: Color,
) {
    parent.spawn((
        Text::new(label),
        font(size),
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
                catalogue
                    .0
                    .materials
                    .iter()
                    .filter(|m| m.solid || m.fluid.is_some())
                    .count()
                    + usize::from(catalogue.0.anything_burns())
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
        Has<bevy::ui::InteractionDisabled>,
        Has<Quiet>,
        Has<EmptyBinding>,
    ),
>;
fn button_label_colors(
    buttons: ButtonLabels,
    mut labels: Query<&mut TextColor>,
    mut images: Query<&mut ImageNode>,
) {
    for (style, hovered, children, disabled, quiet, empty) in &buttons {
        let color = if disabled {
            theme::DISABLED_INK
        } else if empty {
            theme::MUTED
        } else if style.primary {
            theme::SAGE_INK
        } else if (quiet || style.tab && !style.selected) && !hovered.0 {
            theme::MUTED
        } else if style.selected && !style.tab {
            theme::SEGMENT_ON_INK
        } else {
            theme::INK
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tone {
    /// Something the Player asked for took effect.
    Change,
    /// Something was refused or failed.
    Alert,
}
impl Tone {
    fn accent(self) -> Color {
        match self {
            Self::Change => theme::ACCENT,
            Self::Alert => theme::WARN,
        }
    }
}
const NOTICE_INSET_PX: f32 = 16.0;
const NOTICE_SLIDE_S: f32 = 0.22;
/// Past the widest card, so the slide starts fully off screen.
const NOTICE_OFFSCREEN_PX: f32 = -460.0;

/// The card's right offset: eases in from off screen, rests, then eases back out.
fn notice_right_px(elapsed_s: f32, lifetime_s: f32) -> f32 {
    let shown = (elapsed_s.min(lifetime_s - elapsed_s) / NOTICE_SLIDE_S).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - shown).powi(3);
    NOTICE_OFFSCREEN_PX + (NOTICE_INSET_PX - NOTICE_OFFSCREEN_PX) * eased
}
pub(super) fn notify(commands: &mut Commands, message: &str, tone: Tone) {
    notify_for(commands, message, tone, 2.6);
}
pub(super) fn notify_for(commands: &mut Commands, message: &str, tone: Tone, seconds: f32) {
    let accent = tone.accent();
    commands.queue(|world: &mut World| {
        let previous: Vec<_> = world
            .query_filtered::<Entity, With<Notification>>()
            .iter(world)
            .collect();
        for entity in previous {
            world.despawn(entity);
        }
    });
    commands
        .spawn((
            Notification(Timer::from_seconds(seconds, TimerMode::Once)),
            GlobalZIndex(40),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(NOTICE_OFFSCREEN_PX),
                top: Val::Px(NOTICE_INSET_PX),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            root.spawn((
                BackgroundColor(theme::NOTICE_CARD),
                BorderColor {
                    left: accent,
                    ..BorderColor::all(theme::GROUT)
                },
                BoxShadow::new(
                    Color::BLACK.with_alpha(0.53),
                    Val::Px(0.0),
                    Val::Px(10.0),
                    Val::Px(0.0),
                    Val::Px(30.0),
                ),
                Node {
                    max_width: Val::Px(360.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(11.0)),
                    border: UiRect {
                        left: Val::Px(4.0),
                        ..UiRect::all(Val::Px(theme::FRAME))
                    },
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|card| {
                card.spawn(theme::ring(theme::RADIUS_MENU));
                paragraph(card, message, theme::TEXT_CAPTION, theme::INK);
            });
        });
}
fn age_notifications(
    mut commands: Commands,
    time: Option<Res<Time>>,
    mut notifications: Query<(Entity, &mut Notification, &mut Node)>,
    preferences: Option<Res<super::profile::Preferences>>,
) {
    let Some(time) = time else {
        return;
    };
    let still = preferences.is_some_and(|p| p.reduced_motion);
    for (entity, mut notice, mut node) in &mut notifications {
        if notice.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        } else if still {
            node.right = Val::Px(NOTICE_INSET_PX);
        } else {
            node.right = Val::Px(notice_right_px(
                notice.0.elapsed_secs(),
                notice.0.duration().as_secs_f32(),
            ));
        }
    }
}

pub(super) fn description(parent: &mut ChildSpawnerCommands, value: &str) {
    parent.spawn((
        Text::new(value),
        font(theme::TEXT_SMALL),
        bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
        TextColor(theme::MUTED),
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
        bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
        TextColor(color),
        Pickable::IGNORE,
    ));
}
fn dashed_border_colors(
    mut borders: Query<(&EmptyBorder, &mut ImageNode)>,
    buttons: Query<&bevy::picking::hover::Hovered>,
    focus: Res<InputFocus>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let focused = visible_focus(Some(&focus), visible.as_deref());
    for (border, mut image) in &mut borders {
        image.color =
            if focused == Some(border.0) || buttons.get(border.0).is_ok_and(|hovered| hovered.0) {
                theme::SAGE
            } else {
                theme::EDGE
            };
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn notifications_slide_in_from_the_right_and_back_out() {
        use super::{NOTICE_INSET_PX, NOTICE_OFFSCREEN_PX, NOTICE_SLIDE_S, notice_right_px};
        assert_eq!(notice_right_px(0.0, 2.6), NOTICE_OFFSCREEN_PX);
        let entering = notice_right_px(NOTICE_SLIDE_S / 2.0, 2.6);
        assert!(NOTICE_OFFSCREEN_PX < entering && entering < NOTICE_INSET_PX);
        assert_eq!(notice_right_px(NOTICE_SLIDE_S, 2.6), NOTICE_INSET_PX);
        assert_eq!(notice_right_px(1.3, 2.6), NOTICE_INSET_PX);
        assert_eq!(notice_right_px(2.6, 2.6), NOTICE_OFFSCREEN_PX);
    }

    #[test]
    fn alerts_and_changes_carry_distinct_accents() {
        for (tone, accent) in [
            (super::Tone::Change, theme::ACCENT),
            (super::Tone::Alert, theme::WARN),
        ] {
            let mut app = App::new();
            super::notify(&mut app.world_mut().commands(), "Notice", tone);
            app.world_mut().flush();
            assert!(
                app.world_mut()
                    .query::<&BorderColor>()
                    .iter(app.world())
                    .any(|color| color.left == accent)
            );
        }
    }

    #[test]
    fn reduced_motion_shows_a_notice_in_place_without_sliding() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .insert_resource(super::super::profile::Preferences {
                reduced_motion: true,
                ..default()
            })
            .add_systems(Update, super::age_notifications);
        super::notify(
            &mut app.world_mut().commands(),
            "Saved",
            super::Tone::Change,
        );
        app.world_mut().flush();
        app.update();
        let right = app
            .world_mut()
            .query_filtered::<&Node, With<super::Notification>>()
            .single(app.world())
            .unwrap()
            .right;
        assert_eq!(right, Val::Px(super::NOTICE_INSET_PX));
    }

    #[test]
    fn notifications_replace_previous_messages_and_expire() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_systems(Update, super::age_notifications);
        super::notify(
            &mut app.world_mut().commands(),
            "Physics mode",
            super::Tone::Change,
        );
        super::notify(
            &mut app.world_mut().commands(),
            "Free flight mode",
            super::Tone::Change,
        );
        app.world_mut().flush();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<super::Notification>>()
                .iter(app.world())
                .count(),
            1
        );
        let messages: Vec<_> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.as_str())
            .collect();
        assert_eq!(messages, ["Free flight mode"]);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(3));
        app.update();
        assert_eq!(
            app.world_mut().query::<&Text>().iter(app.world()).count(),
            0
        );
    }

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
        let mut settings = app.world_mut().resource_mut::<Settings>();
        settings.open = true;
        settings.tab = Tab::Bindings;
        app
    }
    fn field(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, With<SearchField>>()
            .single(app.world())
            .unwrap()
    }
    #[test]
    fn the_rail_marks_the_open_category_and_quiet_buttons_stay_borderless() {
        let mut app = app();
        app.add_systems(Update, (style, button_label_colors).chain().after(shell));
        app.update();
        let looks: Vec<(Tab, bool)> = app
            .world_mut()
            .query::<(&Operation, &super::super::cinematic::Look)>()
            .iter(app.world())
            .filter_map(|(op, look)| match op {
                Operation::Navigate(tab) => Some((*tab, look.selected)),
                _ => None,
            })
            .collect();
        assert_eq!(looks.len(), Tab::ORDER.len());
        for (tab, selected) in looks {
            assert_eq!(selected, tab == Tab::Bindings, "{tab:?}");
        }
        let parent = app.world_mut().spawn(Node::default()).id();
        app.world_mut()
            .commands()
            .entity(parent)
            .with_children(|children| {
                buttons::button(
                    children,
                    "Restore defaults",
                    Operation::Reset,
                    ButtonStyle::Quiet { text_px: 10.0 },
                );
            });
        app.world_mut().flush();
        app.update();
        let quiet = app
            .world_mut()
            .query_filtered::<Entity, With<Quiet>>()
            .iter(app.world())
            .find(|entity| {
                app.world()
                    .get::<Operation>(*entity)
                    .is_some_and(|op| matches!(op, Operation::Reset))
            })
            .unwrap();
        assert_eq!(
            app.world().get::<BackgroundColor>(quiet).unwrap().0,
            Color::NONE
        );
        assert_eq!(
            *app.world().get::<BorderColor>(quiet).unwrap(),
            BorderColor::all(Color::NONE)
        );
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
        assert_eq!(app.world().resource::<Searches>().bindings, "Stone");
        app.world_mut().resource_mut::<Settings>().tab = Tab::Display;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&SearchField>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<Settings>().tab = Tab::Bindings;
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
