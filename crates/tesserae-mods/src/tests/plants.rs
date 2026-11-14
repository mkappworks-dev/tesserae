//! A column's `stands-in` names a fluid Material, and resolves to it.

use tesserae_core::plant::Kind;
use tesserae_core::{MaterialId, Tables};

use super::behaviour::{empty, fluid, material};
use crate::bindings::tesserae::mods::content;
use crate::{Behaviour, DeclaredPlants, Kind as NameKind, ModError, Registry, apply, resolve};

fn column(stands_in: Option<&str>) -> content::PlantDef {
    content::PlantDef {
        name: "kelp".into(),
        seed: "t:kelp".into(),
        needs: content::PlantNeeds {
            soil: vec!["t:sand".into()],
            light_min: None,
            light_max: None,
            temperature: None,
            humidity: None,
            water: None,
        },
        kind: content::PlantKind::Column(content::ColumnDef {
            material: "t:kelp".into(),
            max_height_m: 8.0,
            direction: content::ColumnDirection::Up,
            mean_seconds: 32.0,
            stands_in: stands_in.map(Into::into),
            yields: Vec::new(),
        }),
    }
}

fn resolved(stands_in: Option<&str>) -> Result<(Tables, Registry, Vec<Kind>), ModError> {
    let mut d = empty();
    d.materials = vec![
        material("sand", true),
        material("kelp", true),
        material("sea", false),
    ];
    d.fluids = vec![fluid("t:sea")];
    d.plants = vec![column(stands_in)];
    let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
    let mut behaviours: Vec<Behaviour> =
        vec![apply(d, "t", &mut tables, &mut registry, false)?.behaviour];
    let declared = DeclaredPlants::take(&mut behaviours);
    resolve(behaviours, &mut tables, &registry)?;
    let plants = declared.resolve(&registry, &tables)?;
    let kinds = plants.plants().iter().map(|p| p.kind.clone()).collect();
    Ok((tables, registry, kinds))
}

fn id(registry: &Registry, name: &str) -> MaterialId {
    registry.id_of(NameKind::Material, name).unwrap() as MaterialId
}

#[test]
fn a_columns_stands_in_resolves_to_the_fluid_material() {
    let (_, registry, kinds) = resolved(Some("t:sea")).unwrap();
    let [Kind::Column { stands_in, .. }] = &kinds[..] else {
        panic!("one column");
    };
    assert_eq!(*stands_in, Some(id(&registry, "t:sea")));
}

#[test]
fn a_column_that_stands_in_nothing_grows_in_air() {
    let (_, _, kinds) = resolved(None).unwrap();
    assert!(matches!(
        kinds[..],
        [Kind::Column {
            stands_in: None,
            ..
        }]
    ));
}

#[test]
fn stands_in_must_name_a_fluid_material_that_exists() {
    let err = resolved(Some("t:sand")).unwrap_err().to_string();
    assert!(
        err.contains("kelp") && err.contains("fluid Material"),
        "{err}"
    );
    let err = resolved(Some("t:nothing")).unwrap_err().to_string();
    assert!(err.contains("kelp") && err.contains("t:nothing"), "{err}");
}

#[test]
fn a_data_mod_declares_stands_in_in_its_column_row() {
    let toml = |extra: &str| {
        format!(
            "[[plants]]\nname = \"kelp\"\nkind = \"column\"\nseed = \"t:kelp\"\n\
             material = \"t:kelp\"\nmax-height-m = 8.0\ndirection = \"up\"\n\
             mean-seconds = 32.0\n{extra}\n[plants.needs]\nsoil = [\"t:sand\"]\n"
        )
    };
    let parse = |text: String| {
        let payload = crate::trust::CheckedPayload::for_test(
            "content.toml",
            [("content.toml", text.into_bytes())],
        );
        crate::data::parse("t", &payload)
    };
    let declared = parse(toml("stands-in = \"t:sea\"")).unwrap();
    let content::PlantKind::Column(c) = &declared.plants[0].kind else {
        panic!("a column");
    };
    assert_eq!(c.stands_in.as_deref(), Some("t:sea"));
    let air = parse(toml("")).unwrap();
    let content::PlantKind::Column(c) = &air.plants[0].kind else {
        panic!("a column");
    };
    assert_eq!(c.stands_in, None);
}
