//! The one set of interface values every menu, sheet and HUD piece draws from:
//! the approved prototype's final tokens (its last stylesheet in the cascade
//! wins), so a screen never carries its own colour, size or font literal.

use bevy::prelude::*;

const PANEL: Color = Color::srgb_u8(0x0b, 0x0b, 0x0c);
pub(super) const RAISED: Color = Color::srgb_u8(0x18, 0x18, 0x1b);
const SHEET_HEADER: Color = Color::srgb_u8(0x11, 0x11, 0x13);
/// World, Game, Mod and Biome cards.
pub(super) const CARD: Color = Color::srgb_u8(0x19, 0x19, 0x1c);
pub(super) const CHOSEN: Color = Color::srgb_u8(0x27, 0x27, 0x2b);
pub(super) const CHOSEN_EDGE: Color = Color::srgb_u8(0xa8, 0xb0, 0xa2);
const EDGE: Color = Color::srgb_u8(0x64, 0x64, 0x6b);
pub(super) const GROUT: Color = Color::BLACK;
/// The 1 px light ring just inside a panel's black frame.
const RING: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x1f);
pub(super) const INK: Color = Color::srgb_u8(0xf7, 0xf4, 0xec);
const MUTED: Color = Color::srgb_u8(0xc7, 0xc9, 0xce);
/// Rail entries, the edition line and the prompt bar.
const CHROME_INK: Color = Color::srgb_u8(0xe1, 0xe4, 0xe3);
/// The Inventory's and the Catalogue's quieter eyebrows.
const ITEM_EYEBROW: Color = Color::srgb_u8(0xc0, 0xc2, 0xc8);
pub(super) const SAGE: Color = Color::srgb_u8(0xc1, 0xcd, 0xb5);
pub(super) const SAGE_HOVER: Color = Color::srgb_u8(0xd4, 0xde, 0xc9);
pub(super) const SAGE_INK: Color = Color::srgb_u8(0x16, 0x1b, 0x16);
pub(super) const HOVER: Color = Color::srgb_u8(0x53, 0x61, 0x6b);
pub(super) const GHOST: Color = Color::srgba_u8(40, 48, 55, 82);
/// Hover on a borderless glyph or icon button.
pub(super) const QUIET_HOVER: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x14);
pub(super) const QUIET_HOVER_EDGE: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x70);
pub(super) const DANGER: Color = Color::srgb_u8(0xb8, 0x3a, 0x2e);
pub(super) const DANGER_HOVER: Color = Color::srgb_u8(0xcf, 0x45, 0x38);
pub(super) const DANGER_EDGE: Color = Color::srgb_u8(0xb8, 0x3a, 0x2e);
pub(super) const DANGER_INK: Color = Color::WHITE;
pub(super) const DISABLED: Color = Color::srgb_u8(0x29, 0x2c, 0x30);
pub(super) const DISABLED_EDGE: Color = Color::srgb_u8(0x62, 0x66, 0x6c);
pub(super) const DISABLED_INK: Color = Color::srgb_u8(0xb2, 0xb6, 0xbc);
pub(super) const PRIMARY_DISABLED: Color = Color::srgb_u8(0x28, 0x28, 0x2c);
pub(super) const PRIMARY_DISABLED_EDGE: Color = Color::srgb_u8(0x62, 0x62, 0x69);
pub(super) const PRIMARY_DISABLED_INK: Color = Color::srgb_u8(0xb6, 0xb6, 0xbe);
pub(super) const FIELD: Color = Color::srgb_u8(0x13, 0x13, 0x15);
const FIELD_EDGE: Color = Color::srgb_u8(0x79, 0x79, 0x81);
pub(super) const FIELD_HOVER: Color = Color::srgb_u8(0x23, 0x23, 0x27);
pub(super) const FIELD_LOCKED_EDGE: Color = Color::srgb_u8(0x4d, 0x4f, 0x55);
const PLACEHOLDER: Color = Color::srgb_u8(0xae, 0xb2, 0xb8);
pub(super) const NOTICE: Color = RAISED;
const NOTICE_EDGE: Color = Color::srgb_u8(0xa8, 0xb5, 0xbd);
pub(super) const PILL: Color = Color::srgb_u8(0x51, 0x5e, 0x67);
pub(super) const PILL_EDGE: Color = Color::srgb_u8(0x9a, 0xa5, 0xac);
pub(super) const PILL_INK: Color = Color::srgb_u8(0xf0, 0xf2, 0xed);
/// The keyboard focus ring on buttons, fields and rows.
pub(super) const FOCUS: Color = Color::srgb_u8(0xe2, 0xd5, 0xab);
/// The focus ring on checkboxes and slots, which sit tighter.
pub(super) const FOCUS_TIGHT: Color = Color::srgb_u8(0xf3, 0xf6, 0xec);
pub(super) const SEGMENT_ON: Color = Color::srgb_u8(0x35, 0x39, 0x2f);
pub(super) const SEGMENT_ON_EDGE: Color = Color::srgb_u8(0xaa, 0xb3, 0x9e);
pub(super) const SEGMENT_ON_INK: Color = Color::srgb_u8(0xf1, 0xf3, 0xed);
pub(super) const CHECK_EDGE: Color = Color::srgb_u8(0x05, 0x05, 0x06);
pub(super) const CHECK_TOP: Color = Color::srgb_u8(0x1b, 0x1c, 0x1f);
pub(super) const CHECK_BOTTOM: Color = Color::srgb_u8(0x13, 0x14, 0x16);
pub(super) const CHECK_RING: Color = Color::srgb_u8(0x5a, 0x5d, 0x63);
pub(super) const CHECK_RING_ON: Color = Color::srgb_u8(0xe6, 0xee, 0xdb);
/// A native range input's unfilled track, which the prototype leaves to Chrome.
pub(super) const TRACK: Color = Color::srgb_u8(0x3b, 0x3b, 0x3b);
/// Under the launcher until it has scenery of its own.
pub(super) const SCENE: Color = Color::srgb_u8(0x24, 0x2c, 0x32);
/// Behind every menu when Simple menu background is on.
pub(super) const SOLID_BACKGROUND: Color = Color::srgb_u8(0x10, 0x11, 0x13);
/// The prompt bar's band.
const BAND: Color = Color::srgba_u8(42, 52, 59, 133);
const BAND_RULE: Color = Color::srgba_u8(220, 220, 225, 36);
const KEY_EDGE: Color = Color::srgb_u8(0x62, 0x65, 0x6a);
/// Over the World while the Inventory is open: it dims without the menus'
/// shade so the World stays readable while sorting.
pub(super) const INVENTORY_SCRIM: Color = Color::srgba_u8(3, 4, 5, 0xdb);
/// Under a dialog: the prototype's 0.70 alpha as [`css_dim`] gives it.
const OVERLAY: Color = Color::srgba_u8(0x09, 0x0a, 0x0c, 0xe2);
const OVERLAY_RGB: [u8; 3] = [0x09, 0x0a, 0x0c];
/// The prototype has no stronger dim; this CSS alpha makes it plainly darker than its 0.70.
const STRONGER_OVERLAY_ALPHA: f32 = 0.85;
/// The shade over the scenery or the World, darkest at the left and the bottom.
const SHADE: [u8; 3] = [0x19, 0x22, 0x28];
/// An amber that marks a refusal or a change still to apply.
pub(super) const WARN: Color = Color::srgb_u8(0xd6, 0xa9, 0x5a);
pub(super) const WARNING: Color = Color::srgb_u8(0xf0, 0xc0, 0x80);
/// The notice card's accent for a change that took effect.
pub(super) const ACCENT: Color = Color::srgb_u8(0xa8, 0xb5, 0xa2);
const NOTICE_CARD: Color = Color::srgba_u8(0x0b, 0x0b, 0x0d, 0xe6);

/// A picked Game or Biome card, marked by a bar along one edge.
pub(super) const CARD_CHOSEN: Color = Color::srgb_u8(0x29, 0x29, 0x2e);
pub(super) const CARD_CHOSEN_EDGE: Color = Color::srgb_u8(0xc0, 0xc7, 0xbb);
pub(super) const CARD_BAR: f32 = 3.0;
/// Card fills under the pointer: the prototype's `brightness(1.15)`.
pub(super) const CARD_HOVER: Color = Color::srgb_u8(0x1d, 0x1d, 0x20);
pub(super) const CARD_CHOSEN_HOVER: Color = Color::srgb_u8(0x2f, 0x2f, 0x35);
/// The Game mode control's track, and an unchosen half under the pointer.
pub(super) const MODE: Color = Color::srgb_u8(0x1d, 0x1f, 0x22);
pub(super) const MODE_HOVER: Color = Color::srgb_u8(0x29, 0x2b, 0x2f);
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
/// A Mod's category beside its card.
const CATEGORY_INK: Color = Color::srgb_u8(0xae, 0xb2, 0xb9);
/// A World's or a Game's thumbnail: sky, far hills, a near field, the ground,
/// and the tree against them.
pub(super) const THUMBNAIL: [Color; 4] = [
    Color::srgb_u8(0x7b, 0xab, 0xa0),
    Color::srgb_u8(0x40, 0x5c, 0x3b),
    Color::srgb_u8(0x73, 0x98, 0x5a),
    Color::srgb_u8(0x33, 0x4a, 0x30),
];
pub(super) const THUMBNAIL_TREE: Color = Color::srgb_u8(0x26, 0x3e, 0x2f);

/// A menu sheet's shade, which also lights a focused list item: darker than the prototype's 0.72
/// alpha because the UI blends in linear light.
pub(super) const SHEET_SHADE: Color = Color::srgba_u8(10, 11, 12, 230);
/// The sheet's shade at its far edge.
pub(super) const SHEET_SHADE_FAR: Color = Color::srgba_u8(10, 11, 12, 199);

pub(super) const SLOT_EDGE: Color = Color::srgb_u8(5, 5, 6);
const SLOT_RIM: Color = Color::srgb_u8(58, 60, 65);
pub(super) const SLOT_RIM_HOVER: Color = Color::srgb_u8(154, 159, 147);
pub(super) const SLOT_ACTIVE_EDGE: Color = Color::srgb_u8(238, 243, 228);
pub(super) const SLOT_GLOW: Color = Color::srgba_u8(230, 238, 214, 0x40);

pub(super) const TEXT_TITLE: f32 = 28.0;
pub(super) const TEXT_DIALOG_TITLE: f32 = 22.0;
pub(super) const TEXT_HEADING: f32 = 19.0;
pub(super) const TEXT_BODY: f32 = 16.0;
pub(super) const TEXT_SMALL: f32 = 14.0;
pub(super) const TEXT_CAPTION: f32 = 13.0;
pub(super) const TEXT_EYEBROW: f32 = 12.0;
pub(super) const TEXT_TAG: f32 = 11.0;
pub(super) const TEXT_KEY: f32 = 10.0;
pub(super) const TEXT_FOOTER: f32 = 9.0;
/// 0.2 em of the eyebrow size.
pub(super) const EYEBROW_TRACKING: f32 = 2.4;
pub(super) const SECTION_TRACKING: f32 = 1.44;
pub(super) const LINE_HEIGHT: f32 = 1.6;

/// Stronger UI contrast, a Preference. The tokens below change with it: muted
/// text turns to full ink, fills turn opaque, edges and rules brighten and the
/// focus ring thickens. Every other token is the same in both modes. A static
/// rather than a resource so a token stays a plain call; the screens that are
/// open when the Preference changes are rebuilt.
static STRONGER_CONTRAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(super) fn set_stronger_contrast(on: bool) {
    STRONGER_CONTRAST.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub(super) fn stronger_contrast() -> bool {
    STRONGER_CONTRAST.load(std::sync::atomic::Ordering::Relaxed)
}

macro_rules! by_contrast {
    ($($name:ident, $func:ident, $strong:expr;)*) => {
        $(
            pub(super) fn $func() -> Color {
                if stronger_contrast() { $strong } else { $name }
            }
        )*
        /// Each token with whether its two modes differ.
        #[cfg(test)]
        fn contrast_pairs() -> Vec<(&'static str, Color, Color)> {
            vec![$((stringify!($name), $name, $strong)),*]
        }
    };
}

by_contrast! {
    MUTED, muted, INK;
    CHROME_INK, chrome_ink, INK;
    ITEM_EYEBROW, item_eyebrow, INK;
    CATEGORY_INK, category_ink, INK;
    PLACEHOLDER, placeholder, INK;
    ITEM_SOFT, item_soft, ITEM_INK;
    HUD_QUIET_INK, hud_quiet_ink, INK;
    PANEL, panel, Color::srgb_u8(0x11, 0x11, 0x12);
    SHEET_HEADER, sheet_header, Color::srgb_u8(0x17, 0x19, 0x1c);
    NOTICE_CARD, notice_card, Color::srgb_u8(0x0b, 0x0b, 0x0d);
    FULL_NOTICE, full_notice, Color::srgb_u8(11, 11, 13);
    BAND, band, Color::srgb_u8(42, 52, 59);
    OVERLAY, overlay, css_dim(OVERLAY_RGB, STRONGER_OVERLAY_ALPHA);
    EDGE, edge, Color::srgb_u8(0xa2, 0xa6, 0xad);
    FIELD_EDGE, field_edge, Color::srgb_u8(0xb0, 0xb4, 0xbb);
    NOTICE_EDGE, notice_edge, Color::srgb_u8(0xd0, 0xd8, 0xdd);
    KEY_EDGE, key_edge, Color::srgb_u8(0xa2, 0xa6, 0xad);
    SLOT_RIM, slot_rim, Color::srgb_u8(130, 134, 142);
    ITEM_ROW_RULE, item_row_rule, Color::srgb_u8(0x6e, 0x6e, 0x78);
    BAND_RULE, band_rule, Color::srgba_u8(220, 220, 225, 120);
    RING, ring_color, Color::srgba_u8(0xff, 0xff, 0xff, 0x59);
    HUD_ROW_RING, hud_row_ring, Color::srgba_u8(0xff, 0xff, 0xff, 0x59);
}

/// A focus outline's width, thicker under stronger contrast.
pub(super) fn focus_width(base: f32) -> f32 {
    thicker(base, stronger_contrast())
}

fn thicker(base: f32, stronger: bool) -> f32 {
    if stronger { base + 2.0 } else { base }
}

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
pub(super) const SCREEN_PAD_X: f32 = 34.0;

pub(super) const RADIUS_PANEL: f32 = 8.0;
pub(super) const RADIUS_MENU: f32 = 6.0;
pub(super) const RADIUS_CONTROL: f32 = 5.0;
pub(super) const RADIUS_SMALL: f32 = 4.0;
pub(super) const RADIUS_TAG: f32 = 3.0;
pub(super) const FRAME: f32 = 2.0;
pub(super) const RULE: f32 = 1.0;
const FOCUS_WIDTH: f32 = 3.0;
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

/// `font` with Inter's tabular figures, so a changing number keeps one width
/// per digit. The prototype sets these in a monospace face; the interface
/// bundles none, and Inter's `tnum` gives the same steadiness.
pub(super) fn tabular(font: TextFont) -> TextFont {
    TextFont {
        font_features: bevy::text::FontFeatures::builder()
            .enable(bevy::text::FontFeatureTag::TABULAR_FIGURES)
            .build(),
        ..font
    }
}

/// [`weighted`], in tabular figures.
pub(super) fn figures(size: f32, weight: u16) -> TextFont {
    tabular(weighted(size, weight))
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
        BorderColor::all(ring_color()),
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
    Outline::new(
        Val::Px(focus_width(FOCUS_WIDTH)),
        Val::Px(FOCUS_OFFSET),
        Color::NONE,
    )
}

/// `rgb` at the alpha that darkens a mid-grey backdrop as much as CSS's `css_alpha` does: Bevy UI
/// blends in linear light, a browser in sRGB, so the prototype's alpha alone dims too little.
pub(super) fn css_dim(rgb: [u8; 3], css_alpha: f32) -> Color {
    let linear = |srgb: f32| Color::srgb(srgb, srgb, srgb).to_linear().red;
    let grey = linear(0.5);
    let alpha = rgb
        .iter()
        .map(|&c| {
            let c = f32::from(c) / 255.0;
            let dimmed = linear(css_alpha * c + (1.0 - css_alpha) * 0.5);
            (grey - dimmed) / (grey - linear(c))
        })
        .sum::<f32>()
        / 3.0;
    let [r, g, b] = rgb;
    Color::srgba_u8(r, g, b, 0).with_alpha(alpha)
}

/// The shade over the World or the launcher's backdrop.
pub(super) fn shade() -> BackgroundGradient {
    let at = |alpha: f32| css_dim(SHADE, alpha);
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
/// The space around a launcher or Settings screen, above the prompt bar.
pub(super) const SCREEN_PAD_TOP: f32 = 28.0;
pub(super) const SCREEN_PAD_BOTTOM: f32 = 22.0;
pub(super) const WORKSPACE_MAX: f32 = 1180.0;
pub(super) const RAIL_WIDTH: f32 = 224.0;
pub(super) const RAIL_ENTRY_HEIGHT: f32 = 48.0;
/// "You are here" on the World, under the Menu.
pub(super) const STATUS_INK: Color = Color::srgb_u8(0xee, 0xf1, 0xea);
pub(super) const TEXT_STATUS: f32 = 24.0;

// Pass D: the Inventory, the Catalogue and the quick Layout panel.
pub(super) const ITEM_SURFACE: Color = Color::srgb_u8(0x0b, 0x0b, 0x0d);
const ITEM_ROW_RULE: Color = Color::srgb_u8(0x34, 0x34, 0x3c);
pub(super) const ITEM_INK: Color = Color::srgb_u8(0xf4, 0xf4, 0xed);
/// Hints and the hotbar caption under the grids.
const ITEM_SOFT: Color = Color::srgb_u8(0xad, 0xaf, 0xb9);
pub(super) const ITEM_FULL: Color = Color::srgb_u8(0xe8, 0xd2, 0x9f);
pub(super) const ITEM_CAPTION: Color = Color::srgb_u8(0xa1, 0xa5, 0xaf);
pub(super) const CATALOGUE_SUBTITLE: Color = Color::srgb_u8(0xa9, 0xad, 0xb6);
/// An input the Inventory holds too few of.
pub(super) const RECIPE_SHORT: Color = Color::srgb_u8(0xd9, 0x9a, 0x83);
pub(super) const RADIUS_ROW: f32 = 6.0;
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
pub(super) const PICK_ON: Color = Color::srgb_u8(0x35, 0x39, 0x2f);
pub(super) const PICK_ON_EDGE: Color = Color::srgb_u8(0xaa, 0xb3, 0x9e);
pub(super) const PRESET_ON_GLYPH: Color = Color::srgb_u8(0xed, 0xf1, 0xe7);
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
pub(super) const LAYOUT_ROW_ON: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x0f);
pub(super) const ITEM_TEXT_TITLE: f32 = 23.0;
pub(super) const ITEM_TEXT_EYEBROW: f32 = 11.0;
pub(super) const ITEM_BUTTON_TEXT: f32 = 13.0;
pub(super) const ITEM_BUTTON_HEIGHT: f32 = 38.0;
pub(super) const ITEM_RADIUS: f32 = 9.0;
pub(super) const CLOSE_SIZE: f32 = 40.0;
pub(super) const ICON_SIZE: f32 = 20.0;
/// About one slot gap, so a selected slot's glow never reaches its neighbour.
pub(super) const SLOT_GLOW_BLUR: f32 = 5.0;
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
const HUD_QUIET_INK: Color = Color::srgb_u8(0xc9, 0xcc, 0xd1);
/// The 1 px light ring inside a pickup feed row, a shade quieter than [`RING`].
const HUD_ROW_RING: Color = Color::srgba_u8(0xff, 0xff, 0xff, 0x1a);
/// The Inventory-full notice under the held Item's name.
const FULL_NOTICE: Color = Color::srgba_u8(11, 11, 13, 0xcc);
pub(super) const FULL_NOTICE_EDGE: Color = Color::srgba_u8(0xd6, 0xc8, 0x8f, 0x80);
pub(super) const FULL_NOTICE_INK: Color = Color::srgb_u8(0xea, 0xdf, 0xb4);
pub(super) const NOTICE_CARD_INK: Color = Color::srgb_u8(0xf1, 0xf0, 0xed);
pub(super) const DEATH_SCRIM: [Color; 2] = [
    Color::srgba_u8(0x1c, 0x08, 0x08, 0x4d),
    Color::srgba_u8(0x0a, 0x03, 0x03, 0xa6),
];
pub(super) const DEATH_TITLE: Color = Color::srgb_u8(0xf4, 0xec, 0xe8);
pub(super) const DEATH_CAUSE: Color = Color::srgb_u8(0xd4, 0xcf, 0xcc);
pub(super) const TEXT_DEATH_TITLE: f32 = 46.0;
/// −0.01 em of the death title.
pub(super) const DEATH_TITLE_TRACKING: f32 = -0.46;
/// The first-person arm, drawn unlit under the viewmodel's own camera.
pub(super) const VIEWMODEL_ARM: Color = Color::srgb_u8(0xc9, 0x9b, 0x7b);
/// The cursor hint and the F3 readout.
pub(super) const READOUT: Color = Color::srgba(1.0, 1.0, 1.0, 0.7);
/// CSS `brightness(.5)` halves sRGB values; Bevy UI blends in linear light,
/// where that is black at 1 − 0.5^2.2.
pub(super) const DEATH_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.78);

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;
    use bevy::text::TextPlugin;
    use bevy::ui::UiPlugin;
    use bevy::window::WindowPlugin;

    fn width_of(digits: &str, font: TextFont) -> f32 {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::transform::TransformPlugin,
            bevy::input::InputPlugin,
            bevy::a11y::AccessibilityPlugin,
            bevy::picking::DefaultPickingPlugins,
            bevy::input_focus::InputFocusPlugin,
            WindowPlugin::default(),
            bevy::image::ImagePlugin::default(),
            TextPlugin,
            UiPlugin,
        ));
        app.add_message::<bevy::window::WindowEvent>()
            .add_message::<bevy::picking::backend::PointerHits>()
            .init_asset::<Image>()
            .init_asset::<bevy::image::TextureAtlasLayout>()
            .init_asset::<Mesh>()
            .init_asset::<bevy::shader::Shader>();
        app.world_mut()
            .resource_mut::<Assets<Font>>()
            .insert(
                bevy::asset::AssetId::default(),
                Font::from_bytes(
                    include_bytes!("../../../../assets/fonts/InterVariable.ttf").to_vec(),
                ),
            )
            .expect("the default Font asset can be replaced");
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        app.world_mut().spawn(Camera2d);
        let text = app
            .world_mut()
            .spawn((
                Text::new(digits),
                font,
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ))
            .id();
        for _ in 0..3 {
            app.update();
        }
        app.world().get::<ComputedNode>(text).unwrap().size.x
    }

    #[test]
    fn tabular_figures_give_every_digit_one_width() {
        for font in [figures(14.0, SEMIBOLD), tabular(font(12.0))] {
            let wide = width_of("0000", font.clone());
            let narrow = width_of("1111", font.clone());
            assert!(wide > 0.0, "the text was not laid out");
            assert!(
                (wide - narrow).abs() < 0.01,
                "0000 is {wide} px, 1111 is {narrow} px"
            );
        }
    }

    #[test]
    fn every_contrast_token_differs_between_the_modes() {
        for (name, normal, stronger) in contrast_pairs() {
            assert_ne!(normal, stronger, "{name} is the same in both modes");
        }
    }

    #[test]
    fn stronger_contrast_lifts_text_to_ink_and_makes_fills_opaque() {
        let pairs: std::collections::HashMap<_, _> = contrast_pairs()
            .into_iter()
            .map(|(name, normal, stronger)| (name, (normal, stronger)))
            .collect();
        assert_eq!(pairs["MUTED"].1, INK);
        for fill in ["NOTICE_CARD", "FULL_NOTICE", "BAND"] {
            assert_eq!(pairs[fill].1.alpha(), 1.0, "{fill} is not opaque");
        }
        for edge in ["EDGE", "FIELD_EDGE", "SLOT_RIM"] {
            let (normal, stronger) = pairs[edge];
            let light = |c: Color| c.to_linear().red;
            assert!(light(stronger) > light(normal), "{edge} is not brighter");
        }
    }

    /// `dim` over a mid-grey backdrop, blended in linear light as Bevy UI does, against the
    /// prototype's `css` colour at `css_alpha` blended in sRGB as a browser does.
    fn assert_dims_like_the_prototype(dim: Color, css: [u8; 3], css_alpha: f32) {
        let ours = over_grey(dim);
        let theirs = css.map(|c| css_alpha * f32::from(c) / 255.0 + (1.0 - css_alpha) * 0.5);
        for (got, want) in [ours.red, ours.green, ours.blue].into_iter().zip(theirs) {
            assert!((got - want).abs() < 0.01, "{ours:?} vs {theirs:?}");
        }
    }

    /// `dim` over a mid-grey backdrop, blended in linear light as Bevy UI does.
    fn over_grey(dim: Color) -> Srgba {
        let (over, grey) = (dim.to_linear(), Color::srgb(0.5, 0.5, 0.5).to_linear());
        let a = over.alpha;
        Color::LinearRgba(LinearRgba::rgb(
            a * over.red + (1.0 - a) * grey.red,
            a * over.green + (1.0 - a) * grey.green,
            a * over.blue + (1.0 - a) * grey.blue,
        ))
        .to_srgba()
    }

    #[test]
    fn a_dialogs_overlay_dims_as_much_as_the_prototypes() {
        assert_dims_like_the_prototype(OVERLAY, OVERLAY_RGB, 0.70);
    }

    #[test]
    fn the_stronger_overlay_dims_clearly_more() {
        let pairs: std::collections::HashMap<_, _> = contrast_pairs()
            .into_iter()
            .map(|(name, normal, stronger)| (name, (normal, stronger)))
            .collect();
        let (normal, stronger) = pairs["OVERLAY"];
        assert_dims_like_the_prototype(stronger, OVERLAY_RGB, STRONGER_OVERLAY_ALPHA);
        let (normal, stronger) = (over_grey(normal).green, over_grey(stronger).green);
        assert!(
            normal - stronger > 0.05,
            "mid-grey dims to {stronger} under stronger contrast, {normal} normally"
        );
    }

    #[test]
    fn the_shade_dims_as_much_as_the_prototypes() {
        let BackgroundGradient(layers) = shade();
        let darkest = |layer: &Gradient| match layer {
            Gradient::Linear(linear) => linear.stops[0].color,
            other => panic!("{other:?} is not linear"),
        };
        assert_dims_like_the_prototype(darkest(&layers[0]), SHADE, 0.48);
        assert_dims_like_the_prototype(darkest(&layers[1]), SHADE, 0.24);
    }

    #[test]
    fn the_stronger_focus_ring_is_thicker() {
        assert_eq!(thicker(3.0, false), 3.0);
        assert!(thicker(3.0, true) > 3.0);
    }

    #[test]
    fn proportional_figures_do_not() {
        let wide = width_of("0000", weighted(14.0, SEMIBOLD));
        let narrow = width_of("1111", weighted(14.0, SEMIBOLD));
        assert!(
            wide - narrow > 1.0,
            "{wide} vs {narrow}: the control cannot tell"
        );
    }
}
