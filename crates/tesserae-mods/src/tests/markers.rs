//! Marker kinds resolve across Mods, and every refusal names the Mod, the row and the reason.

use tesserae_core::Tables;
use tesserae_core::marker::Icon;

use super::sounds::empty;
use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::bindings::tesserae::mods::types::Srgba;
use crate::{Applied, MAX_MARKER_KINDS_PER_MOD, ModError, Registry, apply, resolve_markers};

fn kind(name: &str, icon: content::MarkerIcon) -> content::MarkerKindDef {
    content::MarkerKindDef {
        name: name.into(),
        icon,
        color: Srgba {
            r: 224,
            g: 177,
            b: 90,
            a: 255,
        },
    }
}

fn declare(
    mod_id: &str,
    is_root: bool,
    kinds: Vec<content::MarkerKindDef>,
    death: Option<&str>,
) -> Result<Applied, ModError> {
    let mut d: Declaration = empty();
    d.marker_kinds = kinds;
    d.death_marker_kind = death.map(Into::into);
    apply(
        d,
        mod_id,
        &mut Tables::builtin(),
        &mut Registry::builtin(),
        is_root,
    )
}

fn resolved(
    mods: Vec<(&str, bool, Vec<content::MarkerKindDef>, Option<&str>)>,
) -> Result<tesserae_core::marker::MarkerKinds, ModError> {
    let behaviours: Vec<_> = mods
        .into_iter()
        .map(|(id, root, kinds, death)| declare(id, root, kinds, death).unwrap().behaviour)
        .collect();
    resolve_markers(&behaviours)
}

#[test]
fn valid_rows_resolve_in_load_order_with_full_names_and_the_death_kind() {
    let markers = resolved(vec![
        (
            "a",
            false,
            vec![kind("home", content::MarkerIcon::Home)],
            None,
        ),
        (
            "g",
            true,
            vec![kind("grave", content::MarkerIcon::Death)],
            Some("g:grave"),
        ),
    ])
    .unwrap();
    let names: Vec<_> = markers.kinds.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names, ["a:home", "g:grave"]);
    assert_eq!(markers.kinds[0].icon, Icon::Home);
    assert_eq!(markers.kinds[0].color, [224, 177, 90, 255]);
    assert_eq!(markers.death, Some(1));
}

#[test]
fn a_declaration_with_none_has_no_kinds_and_no_death_kind() {
    let markers = resolved(vec![("g", true, vec![], None)]).unwrap();
    assert!(markers.kinds.is_empty());
    assert_eq!(markers.death, None);
}

#[test]
fn a_bad_name_or_a_repeat_names_the_mod_and_the_row() {
    for (rows, needle) in [
        (
            vec![kind("Home", content::MarkerIcon::Home)],
            "marker kind 0 `Home`",
        ),
        (
            vec![
                kind("home", content::MarkerIcon::Home),
                kind("home", content::MarkerIcon::Camp),
            ],
            "marker kind 1 `home`",
        ),
    ] {
        let err = resolved(vec![("g", true, rows, None)]).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("mod `g`") && text.contains(needle), "{text}");
    }
}

#[test]
fn a_death_kind_that_names_no_declared_kind_is_refused() {
    let err = resolved(vec![(
        "g",
        true,
        vec![kind("home", content::MarkerIcon::Home)],
        Some("g:grave"),
    )])
    .unwrap_err();
    assert!(matches!(err, ModError::DeathMarker { .. }));
    assert!(err.to_string().contains("mod `g`") && err.to_string().contains("g:grave"));
}

#[test]
fn a_death_kind_may_name_another_mods_kind() {
    let markers = resolved(vec![
        (
            "a",
            false,
            vec![kind("grave", content::MarkerIcon::Death)],
            None,
        ),
        ("g", true, vec![], Some("a:grave")),
    ])
    .unwrap();
    assert_eq!(markers.death, Some(0));
}

#[test]
fn only_the_game_names_a_death_kind() {
    let err = declare("m", false, vec![], Some("m:grave")).unwrap_err();
    assert!(matches!(
        err,
        ModError::NotAGame {
            field: "death-marker-kind",
            ..
        }
    ));
}

#[test]
fn a_mod_over_the_row_cap_is_refused() {
    let rows = (0..=MAX_MARKER_KINDS_PER_MOD)
        .map(|i| kind(&format!("k{i}"), content::MarkerIcon::Pin))
        .collect();
    let err = declare("g", true, rows, None).unwrap_err();
    assert!(matches!(
        err,
        ModError::TooManyRows {
            what: "marker-kind",
            ..
        }
    ));
}

#[test]
fn a_data_mod_declares_kinds_by_icon_name_and_refuses_an_unknown_icon() {
    let parse = |text: &str| {
        let payload = crate::trust::CheckedPayload::for_test(
            "content.toml",
            [("content.toml", text.as_bytes().to_vec())],
        );
        crate::data::parse("t", &payload)
    };
    let declared = parse(
        "death-marker-kind = \"t:grave\"\n[[marker-kinds]]\nname = \"grave\"\nicon = \"death\"\n\
         color = [217, 100, 92, 255]\n",
    )
    .unwrap();
    assert_eq!(declared.marker_kinds[0].icon, content::MarkerIcon::Death);
    assert_eq!(declared.marker_kinds[0].color.r, 217);
    assert_eq!(declared.death_marker_kind.as_deref(), Some("t:grave"));
    assert!(
        parse("[[marker-kinds]]\nname = \"x\"\nicon = \"glyph\"\ncolor = [0, 0, 0, 255]\n")
            .is_err()
    );
}
