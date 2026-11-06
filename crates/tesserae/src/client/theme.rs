//! The one set of interface values every menu, sheet and HUD piece draws from:
//! the approved prototype's final tokens (its last stylesheet in the cascade
//! wins), so a screen never carries its own colour, size or font literal.

use bevy::prelude::*;

pub(super) const PANEL: Color = Color::srgb_u8(0x0b, 0x0b, 0x0c);
pub(super) const RAISED: Color = Color::srgb_u8(0x18, 0x18, 0x1b);
pub(super) const SHEET_HEADER: Color = Color::srgb_u8(0x11, 0x11, 0x13);
pub(super) const SHEET_FOOTER: Color = PANEL;
/// World, Game, Mod and Biome cards.
pub(super) const TRUST_PANEL: Color = Color::srgb_u8(0x0f, 0x0f, 0x11);
pub(super) const CARD: Color = Color::srgb_u8(0x19, 0x19, 0x1c);
pub(super) const CHOSEN: Color = Color::srgb_u8(0x27, 0x27, 0x2b);
pub(super) const CHOSEN_EDGE: Color = Color::srgb_u8(0xa8, 0xb0, 0xa2);
pub(super) const EDGE: Color = Color::srgb_u8(0x64, 0x64, 0x6b);
pub(super) const GROUT: Color = Color::BLACK;
/// The 1 px light ring just inside a panel's black frame.
pub(super) const RING: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x1f);
pub(super) const INK: Color = Color::srgb_u8(0xf7, 0xf4, 0xec);
pub(super) const MUTED: Color = Color::srgb_u8(0xc7, 0xc9, 0xce);
/// Rail entries, the edition line and the footer.
pub(super) const CHROME_INK: Color = Color::srgb_u8(0xe1, 0xe4, 0xe3);
pub(super) const EYEBROW: Color = Color::srgb_u8(0xe3, 0xe7, 0xdf);
/// The Inventory's and the Catalogue's quieter eyebrows.
pub(super) const ITEM_EYEBROW: Color = Color::srgb_u8(0xc0, 0xc2, 0xc8);
pub(super) const SAGE: Color = Color::srgb_u8(0xc1, 0xcd, 0xb5);
pub(super) const SAGE_HOVER: Color = Color::srgb_u8(0xd4, 0xde, 0xc9);
pub(super) const SAGE_INK: Color = Color::srgb_u8(0x16, 0x1b, 0x16);
pub(super) const HOVER: Color = Color::srgb_u8(0x53, 0x61, 0x6b);
pub(super) const GHOST: Color = Color::srgba_u8(40, 48, 55, 82);
/// Hover on a borderless glyph or icon button.
pub(super) const QUIET_HOVER: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x14);
pub(super) const QUIET_HOVER_EDGE: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x70);
pub(super) const DANGER: Color = Color::srgb_u8(0x30, 0x24, 0x26);
pub(super) const DANGER_EDGE: Color = Color::srgb_u8(0xbc, 0x8e, 0x86);
pub(super) const DANGER_INK: Color = Color::srgb_u8(0xf4, 0xc3, 0xba);
pub(super) const DISABLED: Color = Color::srgb_u8(0x29, 0x2c, 0x30);
pub(super) const DISABLED_EDGE: Color = Color::srgb_u8(0x62, 0x66, 0x6c);
pub(super) const DISABLED_INK: Color = Color::srgb_u8(0xb2, 0xb6, 0xbc);
pub(super) const PRIMARY_DISABLED: Color = Color::srgb_u8(0x28, 0x28, 0x2c);
pub(super) const PRIMARY_DISABLED_EDGE: Color = Color::srgb_u8(0x62, 0x62, 0x69);
pub(super) const PRIMARY_DISABLED_INK: Color = Color::srgb_u8(0xb6, 0xb6, 0xbe);
pub(super) const FIELD: Color = Color::srgb_u8(0x13, 0x13, 0x15);
pub(super) const FIELD_EDGE: Color = Color::srgb_u8(0x79, 0x79, 0x81);
pub(super) const FIELD_HOVER: Color = Color::srgb_u8(0x23, 0x23, 0x27);
pub(super) const FIELD_LOCKED: Color = Color::srgb_u8(0x12, 0x13, 0x15);
pub(super) const FIELD_LOCKED_EDGE: Color = Color::srgb_u8(0x4d, 0x4f, 0x55);
pub(super) const FIELD_LOCKED_INK: Color = Color::srgb_u8(0xb2, 0xb6, 0xbd);
pub(super) const PLACEHOLDER: Color = Color::srgb_u8(0xae, 0xb2, 0xb8);
pub(super) const NOTICE: Color = RAISED;
pub(super) const NOTICE_EDGE: Color = Color::srgb_u8(0xa8, 0xb5, 0xbd);
pub(super) const PILL: Color = Color::srgb_u8(0x51, 0x5e, 0x67);
pub(super) const PILL_EDGE: Color = Color::srgb_u8(0x9a, 0xa5, 0xac);
pub(super) const PILL_INK: Color = Color::srgb_u8(0xf0, 0xf2, 0xed);
/// The keyboard focus ring on buttons, fields and rows.
pub(super) const FOCUS: Color = Color::srgb_u8(0xe2, 0xd5, 0xab);
/// The focus ring on checkboxes and slots, which sit tighter.
pub(super) const FOCUS_TIGHT: Color = Color::srgb_u8(0xf3, 0xf6, 0xec);
pub(super) const MENU: Color = Color::srgb_u8(0x15, 0x15, 0x17);
pub(super) const MENU_EDGE: Color = Color::srgb_u8(0x82, 0x82, 0x8a);
pub(super) const OPTION_HOVER: Color = Color::srgb_u8(0x30, 0x30, 0x35);
pub(super) const OPTION_HOVER_EDGE: Color = Color::srgb_u8(0x77, 0x77, 0x80);
pub(super) const OPTION_CHOSEN_INK: Color = Color::srgb_u8(0xdc, 0xe6, 0xd1);
pub(super) const SEGMENT_ON: Color = Color::srgb_u8(0x35, 0x39, 0x2f);
pub(super) const SEGMENT_ON_EDGE: Color = Color::srgb_u8(0xaa, 0xb3, 0x9e);
pub(super) const SEGMENT_ON_INK: Color = Color::srgb_u8(0xf1, 0xf3, 0xed);
pub(super) const CHECK_EDGE: Color = Color::srgb_u8(0x05, 0x05, 0x06);
pub(super) const CHECK_TOP: Color = Color::srgb_u8(0x1b, 0x1c, 0x1f);
pub(super) const CHECK_BOTTOM: Color = Color::srgb_u8(0x13, 0x14, 0x16);
pub(super) const CHECK_RING: Color = Color::srgb_u8(0x5a, 0x5d, 0x63);
pub(super) const CHECK_RING_HOVER: Color = Color::srgb_u8(0x9a, 0x9f, 0x93);
pub(super) const CHECK_RING_ON: Color = Color::srgb_u8(0xe6, 0xee, 0xdb);
/// A native range input's unfilled track, which the prototype leaves to Chrome.
pub(super) const TRACK: Color = Color::srgb_u8(0xef, 0xef, 0xef);
/// Under the launcher until it has scenery of its own.
pub(super) const SCENE: Color = Color::srgb_u8(0x24, 0x2c, 0x32);
/// Behind every menu when Simple menu background is on.
pub(super) const SOLID_BACKGROUND: Color = Color::srgb_u8(0x10, 0x11, 0x13);
/// The launcher's header and footer bands.
pub(super) const BAND: Color = Color::srgba_u8(42, 52, 59, 133);
pub(super) const BAND_RULE: Color = Color::srgba_u8(220, 220, 225, 36);
pub(super) const KEY_EDGE: Color = Color::srgb_u8(0x62, 0x65, 0x6a);
/// Over the World while the Inventory is open: it dims without the menus'
/// shade so the World stays readable while sorting.
pub(super) const INVENTORY_SCRIM: Color = Color::srgba_u8(3, 4, 5, 0xdb);
/// Under a dialog.
pub(super) const OVERLAY: Color = Color::srgba_u8(0x09, 0x0a, 0x0c, 0xb3);
/// The shade over the scenery or the World, darkest at the left and the bottom.
const SHADE: [u8; 3] = [0x19, 0x22, 0x28];
/// An amber that marks a refusal or a change still to apply.
pub(super) const WARN: Color = Color::srgb_u8(0xd6, 0xa9, 0x5a);
pub(super) const WARNING: Color = Color::srgb_u8(0xf0, 0xc0, 0x80);
/// The notice card's accent for a change that took effect.
pub(super) const ACCENT: Color = Color::srgb_u8(0xa8, 0xb5, 0xa2);
pub(super) const NOTICE_CARD: Color = Color::srgba_u8(0x0b, 0x0b, 0x0d, 0xe6);

/// The title screen's words, straight on the scenery.
pub(super) const HERO_INK: Color = Color::srgb_u8(0xd1, 0xd1, 0xd0);
pub(super) const HERO_EYEBROW: Color = Color::srgb_u8(0xdd, 0xde, 0xda);
/// The launcher header's Settings button, which sits on the band unfilled.
pub(super) const ICON_EDGE: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x38);
pub(super) const ICON_INK: Color = Color::srgb_u8(0xe8, 0xeb, 0xe4);
/// A picked Game or Biome card, marked by a bar along one edge.
pub(super) const CARD_CHOSEN: Color = Color::srgb_u8(0x29, 0x29, 0x2e);
pub(super) const CARD_CHOSEN_EDGE: Color = Color::srgb_u8(0xc0, 0xc7, 0xbb);
pub(super) const CARD_BAR: f32 = 3.0;
/// Card fills under the pointer: the prototype's `brightness(1.15)`.
pub(super) const CARD_HOVER: Color = Color::srgb_u8(0x1d, 0x1d, 0x20);
pub(super) const CARD_CHOSEN_HOVER: Color = Color::srgb_u8(0x2f, 0x2f, 0x35);
/// The game mode cards.
pub(super) const MODE: Color = Color::srgb_u8(0x15, 0x16, 0x1a);
pub(super) const MODE_HOVER_EDGE: Color = Color::srgb_u8(0x6c, 0x70, 0x76);
pub(super) const MODE_CHOSEN: Color = Color::srgb_u8(0x1d, 0x20, 0x1c);
pub(super) const MODE_CHOSEN_EDGE: Color = Color::srgb_u8(0xe9, 0xee, 0xe0);
/// A slider's reading.
pub(super) const VALUE_INK: Color = Color::srgb_u8(0xe6, 0xeb, 0xdf);
/// The climate map: cold to warm left to right, wet to dry top to bottom.
pub(super) const CLIMATE_COLD: Color = Color::srgb_u8(0x4d, 0x74, 0x70);
pub(super) const CLIMATE_WARM: Color = Color::srgb_u8(0x67, 0x92, 0x60);
pub(super) const CLIMATE_DRY: Color = Color::srgba_u8(0xa7, 0x80, 0x44, 0xaa);
pub(super) const CLIMATE_EDGE: Color = Color::srgb_u8(0x69, 0x81, 0x6a);
pub(super) const DOT_EDGE: Color = Color::srgb_u8(0x0b, 0x0c, 0x0d);
pub(super) const DOT_RING: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x55);
pub(super) const DOT_GLOW: Color = Color::srgba_u8(0, 0, 0, 0xaa);
pub(super) const TAG_INK: Color = Color::srgb_u8(0xf3, 0xf5, 0xee);
/// The band of ground under a Biome's swatch.
pub(super) const GROUND: Color = Color::srgb_u8(0x55, 0x4d, 0x3c);
/// A Mod's category beside its card.
pub(super) const CATEGORY_INK: Color = Color::srgb_u8(0xae, 0xb2, 0xb9);
/// A World's or a Game's thumbnail: sky, far hills, a near field, the ground,
/// and the tree against them.
pub(super) const THUMBNAIL: [Color; 4] = [
    Color::srgb_u8(0x7b, 0xab, 0xa0),
    Color::srgb_u8(0x40, 0x5c, 0x3b),
    Color::srgb_u8(0x73, 0x98, 0x5a),
    Color::srgb_u8(0x33, 0x4a, 0x30),
];
pub(super) const THUMBNAIL_TREE: Color = Color::srgb_u8(0x26, 0x3e, 0x2f);

pub(super) const SLOT_EDGE: Color = Color::srgb_u8(5, 5, 6);
pub(super) const SLOT_RIM: Color = Color::srgb_u8(58, 60, 65);
pub(super) const SLOT_RIM_HOVER: Color = Color::srgb_u8(154, 159, 147);
pub(super) const SLOT_ACTIVE_EDGE: Color = Color::srgb_u8(238, 243, 228);
pub(super) const SLOT_GLOW: Color = Color::srgba_u8(230, 238, 214, 0x55);

/// The title screen's headline: 5.5 % of the window's width within these
/// bounds, and larger again on a wide window.
pub(super) const TEXT_HERO: std::ops::RangeInclusive<f32> = 42.0..=74.0;
pub(super) const TEXT_HERO_VW: f32 = 5.5;
pub(super) const TEXT_HERO_WIDE: f32 = 80.0;
pub(super) const WIDE_WINDOW: f32 = 1500.0;
/// -0.045 em.
pub(super) const HERO_TRACKING: f32 = -0.045;
pub(super) const TEXT_PANEL_TITLE: f32 = 31.0;
pub(super) const TEXT_TITLE: f32 = 28.0;
pub(super) const TEXT_DIALOG_TITLE: f32 = 22.0;
pub(super) const TEXT_HEADING: f32 = 19.0;
pub(super) const TEXT_BODY: f32 = 16.0;
pub(super) const TEXT_SMALL: f32 = 14.0;
pub(super) const TEXT_CAPTION: f32 = 13.0;
pub(super) const TEXT_MODE_NAME: f32 = 15.0;
pub(super) const TEXT_MODE_ABOUT: f32 = 12.5;
pub(super) const TEXT_EYEBROW: f32 = 12.0;
pub(super) const TEXT_TAG: f32 = 11.0;
pub(super) const TEXT_KEY: f32 = 10.0;
pub(super) const TEXT_FOOTER: f32 = 9.0;
pub(super) const TITLE_TRACKING: f32 = -0.7;
/// 0.2 em of the eyebrow size.
pub(super) const EYEBROW_TRACKING: f32 = 2.4;
pub(super) const SECTION_TRACKING: f32 = 1.44;
pub(super) const LINE_HEIGHT: f32 = 1.6;

pub(super) const REGULAR: u16 = 400;
pub(super) const BOOK: u16 = 450;
pub(super) const MEDIUM: u16 = 500;
pub(super) const LABEL: u16 = 550;
pub(super) const SEMIBOLD: u16 = 600;
pub(super) const STRONG: u16 = 650;
pub(super) const BOLD: u16 = 700;

pub(super) const CONTROL_HEIGHT: f32 = 44.0;
pub(super) const BUTTON_PAD_X: f32 = 15.0;
pub(super) const BUTTON_PAD_Y: f32 = 9.0;
pub(super) const SMALL_PAD_X: f32 = 12.0;
pub(super) const SMALL_PAD_Y: f32 = 8.0;
pub(super) const SHEET_PAD_X: f32 = 26.0;
pub(super) const SCREEN_PAD_X: f32 = 34.0;

pub(super) const RADIUS_PANEL: f32 = 8.0;
pub(super) const RADIUS_MENU: f32 = 6.0;
pub(super) const RADIUS_CONTROL: f32 = 5.0;
pub(super) const RADIUS_SMALL: f32 = 4.0;
pub(super) const RADIUS_TAG: f32 = 3.0;
pub(super) const FRAME: f32 = 2.0;
pub(super) const RULE: f32 = 1.0;
pub(super) const FOCUS_WIDTH: f32 = 3.0;
pub(super) const FOCUS_OFFSET: f32 = 3.0;

/// One variable face, so every weight the theme names renders exactly.
pub(super) const BODY_FAMILY: &str = "Inter Variable";
/// The wordmark's, and nothing else's.
pub(super) const BRAND_FAMILY: &str = "Aileron";

pub(super) fn font(size: f32) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::family(BODY_FAMILY),
        font_size: bevy::text::FontSize::Px(size),
        font_variations: optical(size),
        ..default()
    }
}

/// Inter's optical size for text at `size`, as a browser picks it
/// (`font-optical-sizing: auto`): its display cut is tighter at large sizes.
pub(super) fn optical(size: f32) -> bevy::text::FontVariations {
    bevy::text::FontVariations::builder()
        .set(
            bevy::text::FontVariationTag::OPTICAL_SIZE,
            size.clamp(14.0, 32.0),
        )
        .build()
}

pub(super) fn weighted(size: f32, weight: u16) -> TextFont {
    TextFont {
        weight: bevy::text::FontWeight(weight),
        ..font(size)
    }
}

/// A panel's drop shadow; its inner ring is [`ring`].
pub(super) fn shadow() -> BoxShadow {
    BoxShadow::new(
        Color::BLACK.with_alpha(0.6),
        Val::Px(0.0),
        Val::Px(20.0),
        Val::Px(0.0),
        Val::Px(60.0),
    )
}

/// The light ring inside a 2 px black frame. Bevy's `BoxShadow` has no inset,
/// so it is a child that fills the padding box and draws only its border.
pub(super) fn ring(outer_radius: f32) -> impl Bundle {
    (
        BorderColor::all(RING),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px((outer_radius - FRAME).max(0.0))),
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// A hidden focus ring, lit by whatever paints the control's states.
pub(super) fn focus_ring() -> Outline {
    Outline::new(Val::Px(FOCUS_WIDTH), Val::Px(FOCUS_OFFSET), Color::NONE)
}

/// The shade over the World or the launcher's backdrop.
pub(super) fn shade() -> BackgroundGradient {
    let [r, g, b] = SHADE;
    let at = |alpha: f32| Color::srgba_u8(r, g, b, (alpha * 255.0).round() as u8);
    BackgroundGradient(vec![
        LinearGradient::to_right(vec![
            ColorStop::new(at(0.48), Val::Percent(0.0)),
            ColorStop::new(at(0.06), Val::Percent(70.0)),
        ])
        .into(),
        LinearGradient::to_top(vec![
            ColorStop::new(at(0.24), Val::Percent(0.0)),
            ColorStop::new(Color::NONE, Val::Percent(40.0)),
        ])
        .into(),
    ])
}

// Pass C: the in-game Menu and Settings.
/// The space around a launcher or Settings screen, between header and footer.
pub(super) const SCREEN_PAD_TOP: f32 = 28.0;
pub(super) const SCREEN_PAD_BOTTOM: f32 = 22.0;
pub(super) const WORKSPACE_MAX: f32 = 1180.0;
pub(super) const WORKSPACE_GAP: f32 = 24.0;
pub(super) const RAIL_WIDTH: f32 = 224.0;
pub(super) const RAIL_ENTRY_HEIGHT: f32 = 48.0;
pub(super) const MENU_WIDTH: f32 = 300.0;
/// "You are here" on the World, under the Menu.
pub(super) const STATUS_INK: Color = Color::srgb_u8(0xee, 0xf1, 0xea);
pub(super) const TEXT_STATUS: f32 = 24.0;

// Pass D: the Inventory, the Catalogue and the quick Layout panel.
pub(super) const ITEM_SURFACE: Color = Color::srgb_u8(0x0b, 0x0b, 0x0d);
pub(super) const ITEM_FOOTER: Color = Color::srgb_u8(0x0d, 0x0d, 0x10);
pub(super) const ITEM_INSPECTOR: Color = Color::srgb_u8(0x13, 0x13, 0x15);
pub(super) const CATALOGUE_FOOTER: Color = Color::srgb_u8(0x0e, 0x0e, 0x11);
pub(super) const ITEM_RULE: Color = Color::srgb_u8(0x45, 0x45, 0x4c);
pub(super) const ITEM_FOOTER_RULE: Color = Color::srgb_u8(0x55, 0x55, 0x5e);
pub(super) const ITEM_ROW_RULE: Color = Color::srgb_u8(0x34, 0x34, 0x3c);
pub(super) const ITEM_INK: Color = Color::srgb_u8(0xf4, 0xf4, 0xed);
/// Hints and the hotbar caption under the grids.
pub(super) const ITEM_SOFT: Color = Color::srgb_u8(0xad, 0xaf, 0xb9);
pub(super) const ITEM_CAPACITY: Color = Color::srgb_u8(0xb9, 0xbc, 0xc4);
pub(super) const ITEM_FULL: Color = Color::srgb_u8(0xe8, 0xd2, 0x9f);
pub(super) const ITEM_SOURCE: Color = Color::srgb_u8(0x99, 0x9c, 0xa7);
pub(super) const ITEM_TERM: Color = Color::srgb_u8(0xae, 0xb0, 0xb8);
pub(super) const ITEM_VALUE: Color = Color::srgb_u8(0xec, 0xec, 0xec);
pub(super) const ITEM_NOTE: Color = Color::srgb_u8(0xa9, 0xad, 0xb7);
pub(super) const ITEM_CAPTION: Color = Color::srgb_u8(0xa1, 0xa5, 0xaf);
pub(super) const ITEM_EMPTY_ICON: Color = Color::srgb_u8(0x8b, 0x92, 0x9d);
pub(super) const CATALOGUE_SUBTITLE: Color = Color::srgb_u8(0xa9, 0xad, 0xb6);
pub(super) const CATALOGUE_ID: Color = Color::srgb_u8(0x9f, 0xa4, 0xad);
pub(super) const CATALOGUE_LABEL: Color = Color::srgb_u8(0xc3, 0xc6, 0xcc);
pub(super) const BADGE: Color = Color::srgb_u8(0x23, 0x26, 0x20);
pub(super) const BADGE_EDGE: Color = Color::srgb_u8(0x57, 0x5a, 0x53);
pub(super) const BADGE_INK: Color = Color::srgb_u8(0xc2, 0xc9, 0xba);
pub(super) const CLOSE_EDGE: Color = Color::srgb_u8(0x51, 0x51, 0x59);
pub(super) const CLOSE_INK: Color = Color::srgb_u8(0xd9, 0xda, 0xe0);
pub(super) const KEY_CHIP: Color = Color::srgba_u8(0, 0, 0, 0xbb);
pub(super) const KEY_CHIP_EDGE: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x4d);
pub(super) const KEY_CHIP_INK: Color = Color::srgb_u8(0xf2, 0xf3, 0xee);
/// Filter chips and count presets.
pub(super) const CHIP: Color = Color::srgb_u8(0x17, 0x18, 0x1b);
pub(super) const CHIP_EDGE: Color = Color::srgb_u8(0x45, 0x46, 0x4d);
pub(super) const CHIP_INK: Color = Color::srgb_u8(0xc4, 0xc7, 0xcd);
pub(super) const CHIP_ON: Color = Color::srgb_u8(0xe9, 0xee, 0xe0);
pub(super) const CHIP_ON_INK: Color = Color::srgb_u8(0x11, 0x11, 0x11);
pub(super) const PRESET_ON_INK: Color = Color::srgb_u8(0xf3, 0xf5, 0xee);
pub(super) const STEPPER: Color = Color::srgb_u8(0x18, 0x18, 0x1d);
pub(super) const STEPPER_EDGE: Color = Color::srgb_u8(0x65, 0x65, 0x6f);
pub(super) const STEPPER_INK: Color = Color::srgb_u8(0xe4, 0xe6, 0xe4);
pub(super) const STEPPER_VALUE: Color = Color::srgb_u8(0x20, 0x20, 0x27);
pub(super) const SLOT_TOP: Color = Color::srgb_u8(0x1b, 0x1c, 0x1f);
pub(super) const SLOT_BOTTOM: Color = Color::srgb_u8(0x13, 0x14, 0x16);
pub(super) const SLOT_HOVER_TOP: Color = Color::srgb_u8(0x25, 0x27, 0x2b);
pub(super) const SLOT_ACTIVE_TOP: Color = Color::srgb_u8(0x2c, 0x30, 0x2a);
pub(super) const SLOT_ACTIVE_BOTTOM: Color = Color::srgb_u8(0x20, 0x23, 0x20);
pub(super) const SLOT_RECESS: Color = Color::srgba_u8(0, 0, 0, 0x70);
pub(super) const SLOT_LIFT: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x0d);
pub(super) const SLOT_INSPECTED_EDGE: Color = Color::srgb_u8(0xb8, 0xc2, 0xb0);
pub(super) const SLOT_INSPECTED_RIM: Color = Color::srgb_u8(0xc9, 0xd1, 0xbd);
pub(super) const SLOT_PICKED: Color = Color::srgb_u8(0xe7, 0xd9, 0xa8);
pub(super) const SLOT_DROP: Color = Color::srgb_u8(0xd6, 0xc8, 0x8f);
pub(super) const SLOT_NUMBER: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x73);
pub(super) const ITEM_SHADOW_ALPHA: f32 = 0.667;
pub(super) const LAYOUT_SURFACE: Color = Color::srgba_u8(0x0b, 0x0b, 0x0d, 0xef);
pub(super) const LAYOUT_RULE: Color = Color::srgb_u8(0x49, 0x49, 0x51);
pub(super) const METHOD: Color = Color::srgb_u8(0x19, 0x19, 0x1e);
pub(super) const METHOD_EDGE: Color = Color::srgb_u8(0x54, 0x54, 0x5b);
pub(super) const METHOD_INK: Color = Color::srgb_u8(0xb9, 0xbd, 0xc6);
pub(super) const PRESET: Color = Color::srgb_u8(0x1d, 0x1d, 0x22);
pub(super) const PRESET_EDGE: Color = Color::srgb_u8(0x5c, 0x5c, 0x65);
pub(super) const PRESET_INK: Color = Color::srgb_u8(0xc3, 0xc7, 0xcf);
pub(super) const PRESET_ON: Color = Color::srgb_u8(0x2b, 0x30, 0x29);
pub(super) const PRESET_ON_EDGE: Color = Color::srgb_u8(0xbc, 0xc8, 0xaf);
pub(super) const PRESET_ON_GLYPH: Color = Color::srgb_u8(0xed, 0xf1, 0xe7);
pub(super) const AXIS_TAG: Color = Color::srgb_u8(0x29, 0x29, 0x2e);
pub(super) const AXIS_TAG_INK: Color = Color::srgb_u8(0xd2, 0xd7, 0xdd);
pub(super) const AXIS_WAY_INK: Color = Color::srgb_u8(0xc0, 0xc4, 0xcc);
pub(super) const GAP_RULE: Color = Color::srgb_u8(0x44, 0x44, 0x4d);
pub(super) const GAP_NOTE: Color = Color::srgb_u8(0xaf, 0xb4, 0xc0);
pub(super) const TOTALS: Color = Color::srgb_u8(0x20, 0x20, 0x25);
pub(super) const TOTALS_EDGE: Color = Color::srgb_u8(0x52, 0x52, 0x5b);
pub(super) const TOTALS_TERM: Color = Color::srgb_u8(0xa9, 0xb0, 0xbd);
pub(super) const TOTALS_VALUE: Color = Color::srgb_u8(0xe1, 0xe8, 0xd9);
pub(super) const LAYOUT_ERROR: Color = Color::srgb_u8(0x33, 0x25, 0x26);
pub(super) const LAYOUT_ERROR_EDGE: Color = Color::srgb_u8(0xc8, 0x8d, 0x7d);
pub(super) const LAYOUT_ERROR_INK: Color = Color::srgb_u8(0xf1, 0xbc, 0xad);
pub(super) const POINT_EDGE: Color = Color::srgb_u8(0x72, 0x72, 0x7b);
pub(super) const POINT_INK: Color = Color::srgb_u8(0x9f, 0xa5, 0xb2);
pub(super) const POINT_NEXT: Color = Color::srgb_u8(0xdc, 0xc6, 0x9b);
pub(super) const POINT_NEXT_INK: Color = Color::srgb_u8(0xf0, 0xe0, 0xb6);
pub(super) const POINT_DONE: Color = Color::srgb_u8(0xc0, 0xcf, 0xb2);
pub(super) const POINT_DONE_INK: Color = Color::srgb_u8(0x1d, 0x23, 0x1a);
pub(super) const LAYOUT_NOTE: Color = Color::srgb_u8(0xc2, 0xc8, 0xd1);
pub(super) const LAYOUT_STATUS_INK: Color = Color::srgb_u8(0xc3, 0xcc, 0xb9);
pub(super) const ITEM_TEXT_TITLE: f32 = 23.0;
pub(super) const ITEM_TEXT_HEADING: f32 = 19.0;
pub(super) const ITEM_TEXT_NAME: f32 = 22.0;
pub(super) const ITEM_TEXT_EYEBROW: f32 = 11.0;
pub(super) const ITEM_BUTTON_TEXT: f32 = 13.0;
pub(super) const ITEM_BUTTON_HEIGHT: f32 = 38.0;
pub(super) const ITEM_SMALL_PAD_X: f32 = 12.0;
pub(super) const ITEM_SMALL_PAD_Y: f32 = 8.0;
pub(super) const ITEM_RADIUS: f32 = 9.0;
pub(super) const ITEM_WINDOW_HEIGHT: f32 = 640.0;
pub(super) const ITEM_HEADER_HEIGHT: f32 = 70.0;
pub(super) const ITEM_GAP: f32 = 14.0;
pub(super) const CLOSE_SIZE: f32 = 40.0;
pub(super) const ICON_SIZE: f32 = 20.0;
pub(super) const SLOT_GLOW_BLUR: f32 = 14.0;
/// A plain `button`'s hover brightness.
pub(super) const HOVER_LIFT: f32 = 1.15;
pub(super) const STEP_HEIGHT: f32 = 39.0;
pub(super) const STEPPER_VALUE_TEXT: f32 = 15.0;
pub(super) const SLOT_RECESS_PX: f32 = 3.0;
pub(super) const SLOT_LIFT_PX: f32 = 2.0;
pub(super) const GLYPH_SIZE: f32 = 22.0;
pub(super) const TOOL_TRACKING: f32 = 1.2;

// Pass E: the in-play HUD and the death screen.
pub(super) const HUD_BAR: Color = Color::srgba_u8(6, 7, 8, 0xc7);
pub(super) const HUD_INK: Color = Color::srgb_u8(0xf6, 0xf7, 0xf1);
pub(super) const HUD_SHADOW: Color = Color::srgba_u8(0, 0, 0, 0x88);
pub(super) const HUD_SLOT: [Color; 2] = [
    Color::srgb_u8(0x1b, 0x1c, 0x1f),
    Color::srgb_u8(0x13, 0x14, 0x16),
];
pub(super) const HUD_SLOT_HOVER: [Color; 2] = [
    Color::srgb_u8(0x25, 0x27, 0x2b),
    Color::srgb_u8(0x1b, 0x1c, 0x1f),
];
pub(super) const HUD_SLOT_ACTIVE: [Color; 2] = [
    Color::srgb_u8(0x2c, 0x30, 0x2a),
    Color::srgb_u8(0x20, 0x23, 0x20),
];
/// A slot's recess: dark along the top and left, light along the bottom and right.
pub(super) const HUD_BEVEL: [Color; 2] = [
    Color::srgba_u8(0, 0, 0, 0x70),
    Color::srgba_u8(0xff, 0xff, 0xff, 0x0d),
];
pub(super) const HUD_BEVEL_HOVER: [Color; 2] = [
    Color::srgba_u8(0, 0, 0, 0x60),
    Color::srgba_u8(0xff, 0xff, 0xff, 0x14),
];
pub(super) const HUD_SLOT_INNER_GLOW: Color = Color::srgba_u8(0xe6, 0xee, 0xd6, 0x1f);
pub(super) const HUD_SLOT_NUMBER: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x73);
pub(super) const RETICLE: Color = Color::srgb_u8(0xed, 0xf8, 0xdf);
pub(super) const RETICLE_ALPHA: f32 = 0.7;
pub(super) const RETICLE_REFUSED: Color = Color::srgb(1.0, 0.4, 0.35);
pub(super) const TEXT_RETICLE: f32 = 26.0;
pub(super) const HEALTH: Color = Color::srgb_u8(0xc8, 0x64, 0x5a);
pub(super) const HUNGER: Color = Color::srgb_u8(0xc9, 0xa3, 0x5a);
pub(super) const AIR: Color = Color::srgb_u8(0x8f, 0xb6, 0xc9);
pub(super) const VITAL_EMPTY: Color = Color::srgb_u8(0x1a, 0x1b, 0x1e);
/// A vitals tile's raised edge: light along the top and left, dark below.
pub(super) const VITAL_BEVEL: [Color; 2] = [
    Color::srgba_u8(255, 255, 255, 0x2e),
    Color::srgba_u8(0, 0, 0, 0x59),
];
pub(super) const LOW_HEALTH_VIGNETTE: Color = Color::srgba_u8(0x5a, 0x0d, 0x0d, 0x8c);
pub(super) const HIT_FLASH: Color = Color::srgba_u8(0x8a, 0x1d, 0x1d, 0x40);
pub(super) const HUD_QUIET_INK: Color = Color::srgb_u8(0xc9, 0xcc, 0xd1);
/// The 1 px light ring inside a pickup feed row, a shade quieter than [`RING`].
pub(super) const HUD_ROW_RING: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x1a);
/// The Inventory-full notice under the held Item's name.
pub(super) const FULL_NOTICE: Color = Color::srgba_u8(11, 11, 13, 0xcc);
pub(super) const FULL_NOTICE_EDGE: Color = Color::srgba_u8(0xd6, 0xc8, 0x8f, 0x80);
pub(super) const FULL_NOTICE_INK: Color = Color::srgb_u8(0xea, 0xdf, 0xb4);
pub(super) const NOTICE_CARD_INK: Color = Color::srgb_u8(0xf1, 0xf0, 0xed);
pub(super) const DEATH_SCRIM: [Color; 2] = [
    Color::srgba_u8(0x1c, 0x08, 0x08, 0x4d),
    Color::srgba_u8(0x0a, 0x03, 0x03, 0xa6),
];
pub(super) const DEATH_TITLE: Color = Color::srgb_u8(0xf4, 0xec, 0xe8);
pub(super) const DEATH_CAUSE: Color = Color::srgb_u8(0xd4, 0xcf, 0xcc);
pub(super) const DEATH_EYEBROW: Color = Color::srgb_u8(0xd8, 0xa5, 0x9c);
pub(super) const TEXT_DEATH_TITLE: f32 = 46.0;
/// −0.01 em of the death title.
pub(super) const DEATH_TITLE_TRACKING: f32 = -0.46;
/// The cursor hint and the F3 readout.
pub(super) const READOUT: Color = Color::srgba(1.0, 1.0, 1.0, 0.7);
/// CSS `brightness(.5)` halves sRGB values; Bevy UI blends in linear light,
/// where that is black at 1 − 0.5^2.2.
pub(super) const DEATH_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.78);
