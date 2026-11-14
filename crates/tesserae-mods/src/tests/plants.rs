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
        shape: None,
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

#[test]
fn a_data_mod_names_a_plants_and_a_fruits_shape_in_its_rows() {
    let toml = "[[plants]]\nname = \"oak\"\nkind = \"tree\"\nseed = \"t:sapling\"\n\
         shape = \"t:cross\"\nsapling = \"t:sapling\"\ntrunk = \"t:log\"\n\
         canopy = \"t:leaf\"\ntrunk-height-m = [3.0, 5.0]\ncanopy-radius-m = 2.5\n\
         mean-seconds = 60.0\nsapling-yield = [1, 1]\n\
         [plants.fruit]\nstages = [{ material = \"t:a\", mean-seconds = 20.0 }, \
         { material = \"t:b\" }]\ndensity = 0.2\nharvest-to = 0\nshape = \"t:apple\"\n\
         [plants.needs]\nsoil = [\"t:sand\"]\n";
    let payload = crate::trust::CheckedPayload::for_test(
        "content.toml",
        [("content.toml", toml.as_bytes().to_vec())],
    );
    let declared = crate::data::parse("t", &payload).unwrap();
    let row = &declared.plants[0];
    assert_eq!(row.shape.as_deref(), Some("t:cross"));
    let content::PlantKind::Tree(tree) = &row.kind else {
        panic!("a tree");
    };
    assert_eq!(
        tree.fruit.as_ref().unwrap().shape.as_deref(),
        Some("t:apple")
    );
}

mod surface_features {
    use tesserae_core::{SurfacePlantKind, Tables};

    use super::*;
    use crate::resolve_biomes;

    fn needs() -> content::PlantNeeds {
        content::PlantNeeds {
            soil: vec!["t:sand".into()],
            light_min: None,
            light_max: None,
            temperature: None,
            humidity: None,
            water: None,
        }
    }

    fn stage(material: &str, mean_seconds: Option<f32>) -> content::PlantStage {
        content::PlantStage {
            material: material.into(),
            mean_seconds,
            yields: Vec::new(),
        }
    }

    fn row(name: &str, kind: content::PlantKind) -> content::PlantDef {
        content::PlantDef {
            name: name.into(),
            seed: "t:seed".into(),
            shape: None,
            needs: needs(),
            kind,
        }
    }

    fn plants() -> Vec<content::PlantDef> {
        let count = content::CountRange { min: 1, max: 1 };
        vec![
            row(
                "wheat",
                content::PlantKind::Crop(content::CropDef {
                    stages: vec![
                        stage("t:wheat0", Some(20.0)),
                        stage("t:wheat1", Some(20.0)),
                        stage("t:wheat2", None),
                    ],
                    harvest_to: None,
                }),
            ),
            row(
                "oak",
                content::PlantKind::Tree(content::TreeDef {
                    vines: None,
                    sapling: "t:sapling".into(),
                    sapling_yields: Vec::new(),
                    trunk: "t:log".into(),
                    canopy: "t:leaf".into(),
                    trunk_height_m: (3.0, 5.0),
                    canopy_radius_m: 2.5,
                    mean_seconds: 60.0,
                    trunk_yield: Vec::new(),
                    canopy_yield: Vec::new(),
                    sapling_yield: count,
                    fruit: Some(content::FruitDef {
                        stages: vec![stage("t:apple0", Some(20.0)), stage("t:apple1", None)],
                        density: 0.2,
                        harvest_to: 0,
                        shape: None,
                    }),
                }),
            ),
            row(
                "kelp",
                content::PlantKind::Column(content::ColumnDef {
                    material: "t:kelp".into(),
                    max_height_m: 6.0,
                    direction: content::ColumnDirection::Up,
                    mean_seconds: 32.0,
                    stands_in: Some("t:sea".into()),
                    yields: Vec::new(),
                }),
            ),
            row(
                "hanger",
                content::PlantKind::Column(content::ColumnDef {
                    material: "t:hanger".into(),
                    max_height_m: 6.0,
                    direction: content::ColumnDirection::Down,
                    mean_seconds: 32.0,
                    stands_in: None,
                    yields: Vec::new(),
                }),
            ),
            row(
                "vine",
                content::PlantKind::Climber(content::ClimberDef {
                    material: "t:vine".into(),
                    mean_seconds: 32.0,
                    length_m: 4.0,
                    yields: Vec::new(),
                }),
            ),
            row(
                "grass",
                content::PlantKind::Spreader(content::SpreaderDef {
                    material: "t:grass".into(),
                    reach_m: 2.0,
                    mean_seconds: 32.0,
                    density: 4,
                    yields: Vec::new(),
                }),
            ),
        ]
    }

    fn feature(
        material: Option<&str>,
        plant: Option<&str>,
        shape: Option<&str>,
    ) -> content::SurfaceFeature {
        content::SurfaceFeature {
            material: material.map(Into::into),
            plant: plant.map(Into::into),
            shape: shape.map(Into::into),
            spacing_m: 8.0,
        }
    }

    fn resolved(
        features: Vec<content::SurfaceFeature>,
    ) -> Result<(tesserae_core::BiomeTable, Tables, Registry), ModError> {
        resolved_with(features, plants())
    }

    fn resolved_with(
        features: Vec<content::SurfaceFeature>,
        plants: Vec<content::PlantDef>,
    ) -> Result<(tesserae_core::BiomeTable, Tables, Registry), ModError> {
        let mut d = empty();
        d.materials = [
            "sand", "wheat0", "wheat1", "wheat2", "sapling", "log", "leaf", "apple0", "apple1",
            "kelp", "hanger", "vine", "grass",
        ]
        .map(|n| material(n, true))
        .into();
        d.materials.push(material("sea", false));
        d.fluids = vec![fluid("t:sea")];
        let shape = |name: &str, form| content::ShapeDef {
            name: name.into(),
            boxes: vec![crate::bindings::tesserae::mods::types::NodeBox {
                min: (4, 0, 4),
                max: (12, 16, 12),
            }],
            form,
            collision: None,
            connect: None,
        };
        d.shapes = vec![
            shape("cross", Some(content::ShapeForm::Cross)),
            shape("apple", None),
            content::ShapeDef {
                boxes: vec![crate::bindings::tesserae::mods::types::NodeBox {
                    min: (1, 1, 0),
                    max: (15, 15, 2),
                }],
                ..shape("plane", Some(content::ShapeForm::Plane))
            },
        ];
        d.plants = plants;
        d.biomes = vec![content::BiomeDef {
            soundscape: None,
            name: "field".into(),
            temperature: 0.5,
            humidity: 0.5,
            ground: None,
            sky_fog: None,
            cave_scale: None,
            surface_features: features,
        }];
        let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
        let mut behaviours: Vec<Behaviour> =
            vec![apply(d, "t", &mut tables, &mut registry, false)?.behaviour];
        let declared = DeclaredPlants::take(&mut behaviours);
        let rows = crate::biome_rows(&behaviours);
        resolve(behaviours, &mut tables, &registry)?;
        tables.plants = declared.resolve(&registry, &tables)?;
        let (biomes, _) = resolve_biomes(&rows, &tables, &registry)?;
        Ok((biomes, tables, registry))
    }

    fn refused(features: Vec<content::SurfaceFeature>) -> String {
        resolved(features).unwrap_err().to_string()
    }

    fn with_shapes(plant_shape: Option<&str>, fruit_shape: Option<&str>) -> Vec<content::PlantDef> {
        let mut rows = plants();
        for row in &mut rows {
            match (row.name.as_str(), &mut row.kind) {
                ("wheat", _) => row.shape = plant_shape.map(Into::into),
                ("oak", content::PlantKind::Tree(tree)) => {
                    tree.fruit.as_mut().unwrap().shape = fruit_shape.map(Into::into);
                }
                _ => {}
            }
        }
        rows
    }

    #[test]
    fn a_plants_shape_and_its_fruits_resolve_to_registered_shapes_and_freeze_into_the_biome() {
        let (biomes, tables, registry) = resolved_with(
            vec![
                feature(None, Some("t:wheat"), None),
                feature(None, Some("t:oak"), None),
            ],
            with_shapes(Some("t:cross"), Some("t:apple")),
        )
        .unwrap();
        let shape = |n: &str| registry.id_of(NameKind::Shape, n).unwrap() as u8;
        let wheat = &tables.plants.plants()[0];
        assert_eq!(wheat.shape, Some(shape("t:cross")));
        let Kind::Tree(tree) = &tables.plants.plants()[1].kind else {
            panic!("a tree");
        };
        assert_eq!(tree.fruit.as_ref().unwrap().shape, Some(shape("t:apple")));
        let features = biomes.as_slice()[0].surface_features.as_slice();
        assert_eq!(features[0].plant.unwrap().shape, shape("t:cross"));
        let SurfacePlantKind::Tree(frozen) = features[1].plant.unwrap().kind else {
            panic!("a tree");
        };
        assert_eq!(frozen.fruit.unwrap().shape, shape("t:apple"));
    }

    #[test]
    fn a_feature_shape_overrides_the_plants_and_a_plant_naming_none_is_a_cube_as_before() {
        let (biomes, _, registry) = resolved_with(
            vec![feature(None, Some("t:wheat"), Some("t:apple"))],
            with_shapes(Some("t:cross"), None),
        )
        .unwrap();
        let apple = registry.id_of(NameKind::Shape, "t:apple").unwrap() as u8;
        let features = biomes.as_slice()[0].surface_features.as_slice();
        assert_eq!(features[0].plant.unwrap().shape, apple);
        let (biomes, tables, _) = resolved(vec![feature(None, Some("t:wheat"), None)]).unwrap();
        assert_eq!(tables.plants.plants()[0].shape, None);
        let features = biomes.as_slice()[0].surface_features.as_slice();
        assert_eq!(features[0].plant.unwrap().shape, 0);
    }

    #[test]
    fn an_unknown_plant_or_fruit_shape_is_a_load_error_naming_the_plant() {
        for (plant, fruit, who) in [
            (Some("t:nope"), None, "wheat"),
            (None, Some("t:nope"), "oak"),
        ] {
            let err = resolved_with(vec![], with_shapes(plant, fruit))
                .unwrap_err()
                .to_string();
            assert!(err.contains(who) && err.contains("t:nope"), "{err}");
        }
    }

    #[test]
    fn a_feature_names_a_plant_and_the_world_gets_what_generation_needs() {
        let (biomes, _, registry) = resolved(vec![
            feature(None, Some("t:wheat"), Some("t:cross")),
            feature(None, Some("t:oak"), None),
            feature(None, Some("t:kelp"), Some("t:cross")),
            feature(None, Some("t:grass"), None),
        ])
        .unwrap();
        let features = biomes.as_slice()[0].surface_features.as_slice();
        let id = |n: &str| id(&registry, n);
        let cross = registry.id_of(NameKind::Shape, "t:cross").unwrap() as u8;
        let SurfacePlantKind::Stages { materials, len } = features[0].plant.unwrap().kind else {
            panic!("a crop draws from its Stages");
        };
        assert_eq!(
            (&materials[..len as usize], features[0].plant.unwrap().shape),
            (&[id("t:wheat0"), id("t:wheat1"), id("t:wheat2")][..], cross)
        );
        let SurfacePlantKind::Tree(tree) = features[1].plant.unwrap().kind else {
            panic!("a tree");
        };
        let fruit = tree.fruit.unwrap();
        assert_eq!(
            (
                tree.trunk,
                tree.canopy,
                tree.canopy_radius_m,
                fruit.len,
                fruit.density
            ),
            (id("t:log"), id("t:leaf"), 2.5, 2, 0.2)
        );
        assert_eq!(features[1].material, id("t:log"));
        assert!(matches!(
            features[2].plant.unwrap().kind,
            SurfacePlantKind::Column { stands_in: Some(f), .. } if f == id("t:sea")
        ));
        assert!(matches!(
            features[3].plant.unwrap().kind,
            SurfacePlantKind::Stages { len: 1, .. }
        ));
        assert_eq!(features[3].plant.unwrap().shape, 0, "no shape is a cube");
    }

    #[test]
    fn a_feature_naming_both_or_neither_is_refused() {
        let both = refused(vec![feature(Some("t:sand"), Some("t:wheat"), None)]);
        assert!(
            both.contains("surface_features") && both.contains("not both"),
            "{both}"
        );
        let neither = refused(vec![feature(None, None, None)]);
        assert!(neither.contains("a Material or a Plant"), "{neither}");
    }

    #[test]
    fn a_feature_naming_an_unknown_plant_is_refused() {
        let err = refused(vec![feature(None, Some("t:nothing"), None)]);
        assert!(err.contains("t:nothing") && err.contains("`t`"), "{err}");
    }

    /// The oak hangs `t:vine`, whose Shape and soil are as given.
    fn hung(
        vines: content::TreeVines,
        vine_shape: Option<&str>,
        vine_soil: &[&str],
    ) -> Vec<content::PlantDef> {
        let mut rows = plants();
        for row in &mut rows {
            match (row.name.as_str(), &mut row.kind) {
                ("oak", content::PlantKind::Tree(tree)) => tree.vines = Some(vines.clone()),
                ("vine", _) => {
                    row.shape = vine_shape.map(Into::into);
                    row.needs.soil = vine_soil.iter().map(|s| (*s).into()).collect();
                }
                _ => {}
            }
        }
        rows
    }

    fn vines(plant: &str, density: f32, length_m: f32) -> content::TreeVines {
        content::TreeVines {
            plant: plant.into(),
            density,
            length_m,
        }
    }

    #[test]
    fn a_trees_vines_resolve_to_the_climber_and_freeze_into_the_biome() {
        let (biomes, tables, registry) = resolved_with(
            vec![feature(None, Some("t:oak"), None)],
            hung(
                vines("t:vine", 0.3, 6.0),
                Some("t:plane"),
                &["t:sand", "t:leaf"],
            ),
        )
        .unwrap();
        let Kind::Tree(tree) = &tables.plants.plants()[1].kind else {
            panic!("a tree");
        };
        assert_eq!(tree.vines.as_ref().unwrap().plant, "t:vine");
        let SurfacePlantKind::Tree(frozen) = biomes.as_slice()[0].surface_features.as_slice()[0]
            .plant
            .unwrap()
            .kind
        else {
            panic!("a tree");
        };
        let frozen = frozen.vines.unwrap();
        let plane = registry.id_of(NameKind::Shape, "t:plane").unwrap() as u8;
        assert_eq!(
            (
                frozen.material,
                frozen.shape,
                frozen.density,
                frozen.length_m
            ),
            (id(&registry, "t:vine"), plane, 0.3, 6.0)
        );
        let faces: Vec<_> = frozen.orientations.to_vec();
        assert!(faces.iter().all(|&o| o < 24), "{faces:?}");
        let mut distinct = faces.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 4, "four faces, four Orientations");
    }

    #[test]
    fn vines_that_name_no_climber_or_cannot_hold_are_a_load_error_naming_the_tree() {
        let both = ["t:sand", "t:leaf"];
        let refused = |vines, shape, soil: &[&str]| {
            resolved_with(vec![], hung(vines, shape, soil))
                .unwrap_err()
                .to_string()
        };
        for (what, err) in [
            (
                "no Plant",
                refused(vines("t:nothing", 0.3, 6.0), Some("t:plane"), &both),
            ),
            (
                "a crop",
                refused(vines("t:wheat", 0.3, 6.0), Some("t:plane"), &both),
            ),
            ("no shape", refused(vines("t:vine", 0.3, 6.0), None, &both)),
            (
                "a cross",
                refused(vines("t:vine", 0.3, 6.0), Some("t:cross"), &both),
            ),
            (
                "no soil",
                refused(vines("t:vine", 0.3, 6.0), Some("t:plane"), &["t:sand"]),
            ),
            (
                "density",
                refused(vines("t:vine", 0.6, 6.0), Some("t:plane"), &both),
            ),
            (
                "length",
                refused(vines("t:vine", 0.3, 9.0), Some("t:plane"), &both),
            ),
            (
                "no length",
                refused(vines("t:vine", 0.3, 0.0), Some("t:plane"), &both),
            ),
        ] {
            assert!(err.contains("oak"), "{what}: {err}");
        }
    }

    #[test]
    fn a_biome_names_sixteen_features_and_seventeen_is_refused_naming_it() {
        let many = |n: usize| -> Vec<_> {
            (0..n)
                .map(|_| feature(Some("t:sand"), None, None))
                .collect()
        };
        let (biomes, ..) = resolved(many(16)).unwrap();
        assert_eq!(biomes.as_slice()[0].surface_features.as_slice().len(), 16);
        let err = refused(many(17));
        assert!(
            err.contains("field") && err.contains("17") && err.contains("16"),
            "{err}"
        );
    }

    #[test]
    fn a_climber_a_downward_column_and_a_misplaced_shape_are_refused() {
        let climber = refused(vec![feature(None, Some("t:vine"), None)]);
        assert!(climber.contains("climber"), "{climber}");
        let down = refused(vec![feature(None, Some("t:hanger"), None)]);
        assert!(down.contains("growing down"), "{down}");
        let tree = refused(vec![feature(None, Some("t:oak"), Some("t:cross"))]);
        assert!(tree.contains("cubes"), "{tree}");
        let material = refused(vec![feature(Some("t:sand"), None, Some("t:cross"))]);
        assert!(material.contains("shape"), "{material}");
        let shape = refused(vec![feature(None, Some("t:wheat"), Some("t:missing"))]);
        assert!(shape.contains("t:missing"), "{shape}");
    }
}

#[test]
fn a_data_mod_declares_vines_in_its_tree_row() {
    let toml = "[[plants]]\nname = \"jungle\"\nkind = \"tree\"\nseed = \"t:sapling\"\n\
         sapling = \"t:sapling\"\ntrunk = \"t:log\"\ncanopy = \"t:leaf\"\n\
         trunk-height-m = [3.0, 5.0]\ncanopy-radius-m = 2.5\nmean-seconds = 60.0\n\
         sapling-yield = [1, 1]\nvines = { plant = \"t:vine\", density = 0.3, length-m = 6.0 }\n\
         [plants.needs]\nsoil = [\"t:sand\"]\n";
    let payload = crate::trust::CheckedPayload::for_test(
        "content.toml",
        [("content.toml", toml.as_bytes().to_vec())],
    );
    let declared = crate::data::parse("t", &payload).unwrap();
    let content::PlantKind::Tree(tree) = &declared.plants[0].kind else {
        panic!("a tree");
    };
    let vines = tree.vines.as_ref().unwrap();
    assert_eq!(
        (vines.plant.as_str(), vines.density, vines.length_m),
        ("t:vine", 0.3, 6.0)
    );
}

#[test]
fn a_data_mod_names_a_plant_in_a_biomes_surface_feature() {
    let text = "[[biomes]]\nname = \"field\"\ntemperature = 0.5\nhumidity = 0.5\n\
                surface-features = [{ material = \"t:sand\", spacing-m = 8.0 }, \
                { plant = \"t:wheat\", shape = \"t:cross\", spacing-m = 12.0 }]\n";
    let payload = crate::trust::CheckedPayload::for_test(
        "content.toml",
        [("content.toml", text.as_bytes().to_vec())],
    );
    let declared = crate::data::parse("t", &payload).unwrap();
    let [material, plant] = &declared.biomes[0].surface_features[..] else {
        panic!("two features");
    };
    assert_eq!(material.material.as_deref(), Some("t:sand"));
    assert_eq!(material.plant, None);
    assert_eq!(plant.material, None);
    assert_eq!(plant.plant.as_deref(), Some("t:wheat"));
    assert_eq!(plant.shape.as_deref(), Some("t:cross"));
}
