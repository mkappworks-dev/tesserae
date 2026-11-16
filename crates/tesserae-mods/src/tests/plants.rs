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
    assert_eq!(*stands_in, [id(&registry, "t:sea")]);
}

#[test]
fn a_column_that_stands_in_nothing_grows_in_air() {
    let (_, _, kinds) = resolved(None).unwrap();
    assert!(matches!(
        &kinds[..],
        [Kind::Column { stands_in, .. }] if stands_in.is_empty()
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

fn tagged(name: &str, solid: bool, tags: &[&str]) -> content::MaterialDef {
    content::MaterialDef {
        tags: tags.iter().map(|t| (*t).into()).collect(),
        ..material(name, solid)
    }
}

/// Two Mods tag their own soil and water; the kelp names only the tags.
fn resolved_by_tag(
    soil: &str,
    stands_in: &str,
) -> Result<(Registry, Vec<tesserae_core::plant::Plant>), ModError> {
    let mut game = empty();
    game.materials = vec![
        tagged("sand", true, &["soil", "sand"]),
        tagged("kelp", true, &[]),
        tagged("sea", false, &["water"]),
    ];
    game.fluids = vec![fluid("g:sea")];
    let mut plant = column(Some(stands_in));
    plant.seed = "g:kelp".into();
    plant.needs.soil = vec![soil.into()];
    plant.kind = match plant.kind {
        content::PlantKind::Column(c) => content::PlantKind::Column(content::ColumnDef {
            material: "g:kelp".into(),
            ..c
        }),
        other => other,
    };
    game.plants = vec![plant];
    let mut other = empty();
    other.materials = vec![tagged("loam", true, &["soil"])];
    let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
    let mut behaviours: Vec<Behaviour> = vec![
        apply(other, "o", &mut tables, &mut registry, false)?.behaviour,
        apply(game, "g", &mut tables, &mut registry, false)?.behaviour,
    ];
    let declared = DeclaredPlants::take(&mut behaviours);
    resolve(behaviours, &mut tables, &registry)?;
    let plants = declared.resolve(&registry, &tables)?;
    Ok((registry, plants.plants().to_vec()))
}

#[test]
fn a_tag_expands_to_every_material_of_any_mod_that_carries_it() {
    let (registry, plants) = resolved_by_tag("#soil", "#water").unwrap();
    let plant = &plants[0];
    assert_eq!(
        plant.needs.soil,
        [id(&registry, "o:loam"), id(&registry, "g:sand")]
    );
    let Kind::Column { stands_in, .. } = &plant.kind else {
        panic!("a column")
    };
    assert_eq!(*stands_in, [id(&registry, "g:sea")]);
    let (registry, plants) = resolved_by_tag("g:sand", "g:sea").unwrap();
    assert_eq!(plants[0].needs.soil, [id(&registry, "g:sand")]);
}

#[test]
fn a_tag_that_matches_nothing_or_the_wrong_kind_fails_the_load_naming_the_plant() {
    for (soil, water) in [
        ("#nothing", "#water"),
        ("#soil", "#soil"),
        ("#water", "#water"),
    ] {
        let err = resolved_by_tag(soil, water).unwrap_err().to_string();
        assert!(err.contains("kelp") && err.contains("tag `#"), "{err}");
    }
}

mod references {
    use super::*;
    use crate::declare::ItemDef;
    use tesserae_core::plant::PlantTable;

    fn wheat(soil: &str) -> content::PlantDef {
        let stage = |material: &str, mean_seconds| content::PlantStage {
            material: material.into(),
            mean_seconds,
            yields: vec![content::PlantYield {
                item: "t:grain".into(),
                count: content::CountRange { min: 1, max: 1 },
            }],
            shape: None,
        };
        content::PlantDef {
            name: "wheat".into(),
            seed: "t:seed".into(),
            shape: None,
            needs: content::PlantNeeds {
                soil: vec![soil.into()],
                light_min: None,
                light_max: None,
                temperature: None,
                humidity: None,
                water: None,
            },
            kind: content::PlantKind::Crop(content::CropDef {
                stages: vec![stage("t:w0", Some(32.0)), stage("t:w1", None)],
                harvest_to: None,
                fruit_vine: None,
            }),
        }
    }

    fn load(soil: &str) -> Result<(PlantTable, DeclaredPlants, Registry), ModError> {
        let mut d = empty();
        d.materials = ["sand", "w0", "w1"].map(|n| material(n, true)).into();
        d.plants = vec![wheat(soil)];
        let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
        let mut behaviours: Vec<Behaviour> =
            vec![apply(d, "t", &mut tables, &mut registry, false)?.behaviour];
        let declared = DeclaredPlants::take(&mut behaviours);
        resolve(behaviours, &mut tables, &registry)?;
        let table = declared.resolve(&registry, &tables)?;
        Ok((table, declared, registry))
    }

    fn item(name: &str, material: Option<MaterialId>) -> ItemDef {
        ItemDef {
            name: name.into(),
            size_m: 0.25,
            color: [0; 4],
            material,
            max_stack: 64,
            food: None,
            texture: None,
        }
    }

    #[test]
    fn the_engine_ships_no_plant() {
        assert!(Tables::builtin().plants.is_empty());
    }

    #[test]
    fn an_unknown_soil_material_fails_the_load_naming_the_plant() {
        let err = load("t:ghost").unwrap_err().to_string();
        assert!(err.contains("wheat") && err.contains("t:ghost"), "{err}");
    }

    #[test]
    fn a_seed_a_yield_and_a_grown_material_are_checked_against_the_items() {
        let (table, declared, registry) = load("t:sand").unwrap();
        let first = id(&registry, "t:w0");
        let grown = id(&registry, "t:w1");
        let items = |extra: Option<ItemDef>| {
            let mut items = vec![item("t:seed", Some(first)), item("t:grain", None)];
            items.extend(extra);
            items
        };
        assert!(declared.check_items(&table, &items(None)).is_ok());

        let err = declared.check_items(&table, &[item("t:grain", None)]);
        assert!(err.unwrap_err().to_string().contains("seed `t:seed`"));
        let wrong = [item("t:seed", Some(grown)), item("t:grain", None)];
        let err = declared.check_items(&table, &wrong).unwrap_err();
        assert!(err.to_string().contains("first Stage"), "{err}");
        let err = declared.check_items(&table, &[item("t:seed", Some(first))]);
        assert!(err.unwrap_err().to_string().contains("t:grain"));
        let err = declared
            .check_items(&table, &items(Some(item("t:placed", Some(grown)))))
            .unwrap_err();
        assert!(err.to_string().contains("grown Material"), "{err}");
    }
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
         { material = \"t:b\" }]\ndensity = 0.2\nregrow-seconds = 90.0\nshape = \"t:apple\"\n\
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
            shape: None,
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
                    fruit_vine: None,
                }),
            ),
            row(
                "oak",
                content::PlantKind::Tree(content::TreeDef {
                    vines: None,
                    branching: None,
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
                        regrow_seconds: 90.0,
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
                    min_neighbours: None,
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
            clump: None,
            field: None,
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
        resolved_in(features, Vec::new(), plants)
    }

    fn resolved_in(
        features: Vec<content::SurfaceFeature>,
        crop_fields: Vec<content::CropField>,
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
            crop_fields,
            structures: Vec::new(),
            settlements: Vec::new(),
        }];
        let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
        let mut behaviours: Vec<Behaviour> =
            vec![apply(d, "t", &mut tables, &mut registry, false)?.behaviour];
        let declared = DeclaredPlants::take(&mut behaviours);
        let rows = crate::biome_rows(&behaviours);
        resolve(behaviours, &mut tables, &registry)?;
        tables.plants = declared.resolve(&registry, &tables)?;
        let (biomes, _) = resolve_biomes(&rows, &tables, &registry, None)?;
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

    fn with_stage_shapes(crop: Option<&str>, fruit: Option<&str>) -> Vec<content::PlantDef> {
        let mut rows = plants();
        for row in &mut rows {
            match (row.name.as_str(), &mut row.kind) {
                ("wheat", content::PlantKind::Crop(crop_def)) => {
                    crop_def.stages[1].shape = crop.map(Into::into);
                }
                ("oak", content::PlantKind::Tree(tree)) => {
                    tree.fruit.as_mut().unwrap().stages[0].shape = fruit.map(Into::into);
                }
                _ => {}
            }
        }
        rows
    }

    #[test]
    fn a_stage_shape_resolves_and_freezes_by_stage_into_the_biome() {
        let (biomes, tables, registry) = resolved_with(
            vec![feature(None, Some("t:oak"), None)],
            with_stage_shapes(Some("t:cross"), Some("t:apple")),
        )
        .unwrap();
        let shape = |n: &str| registry.id_of(NameKind::Shape, n).unwrap() as u8;
        let Kind::Crop { stages, .. } = &tables.plants.plants()[0].kind else {
            panic!("a crop");
        };
        assert_eq!(stages[0].shape, None);
        assert_eq!(stages[1].shape, Some(shape("t:cross")));
        let Kind::Tree(tree) = &tables.plants.plants()[1].kind else {
            panic!("a tree");
        };
        assert_eq!(
            tree.fruit.as_ref().unwrap().stages[0].shape,
            Some(shape("t:apple"))
        );
        let features = biomes.as_slice()[0].surface_features.as_slice();
        let SurfacePlantKind::Tree(frozen) = features[0].plant.unwrap().kind else {
            panic!("a tree");
        };
        assert_eq!(
            frozen.fruit.unwrap().stage_shapes,
            [Some(shape("t:apple")), None, None, None]
        );
    }

    fn field(crops: &[(&str, f32)]) -> content::CropField {
        content::CropField {
            crops: crops.iter().map(|&(n, w)| (n.to_owned(), w)).collect(),
            plot_m: (8.0, 12.0),
            spacing_m: 64.0,
            path: Some("t:sand".into()),
            ditch: Some("t:sea".into()),
            near_water_m: Some(24.0),
        }
    }

    #[test]
    fn a_crop_field_resolves_its_crops_to_the_stages_worldgen_stands_and_freezes_into_the_biome() {
        let (biomes, tables, registry) = resolved_in(
            vec![],
            vec![field(&[("t:wheat", 5.0)])],
            with_stage_shapes(Some("t:cross"), None),
        )
        .unwrap();
        let id = |n: &str| registry.id_of(NameKind::Material, n).unwrap() as u16;
        let shape = |n: &str| registry.id_of(NameKind::Shape, n).unwrap() as u8;
        let [frozen] = biomes.as_slice()[0].surface_features.crop_fields() else {
            panic!("one field");
        };
        let [crop] = frozen.crops() else {
            panic!("one crop");
        };
        assert_eq!(
            crop.materials[..3],
            [id("t:wheat0"), id("t:wheat1"), id("t:wheat2")]
        );
        assert_eq!((crop.len, crop.weight), (3, 5.0));
        assert_eq!(crop.stage_shapes[..3], [None, Some(shape("t:cross")), None]);
        assert_eq!(crop.shape, tables.plants.plants()[0].shape.unwrap_or(0));
        assert_eq!(
            (frozen.plot_m, frozen.spacing_m, frozen.near_water_m),
            ((8.0, 12.0), 64.0, Some(24.0))
        );
        assert_eq!(
            (frozen.path, frozen.ditch),
            (Some(id("t:sand")), Some(id("t:sea")))
        );
    }

    #[test]
    fn a_bad_crop_field_names_the_biome_and_what_is_wrong() {
        for (bad, says) in [
            (field(&[("t:oak", 1.0)]), "is not a crop"),
            (field(&[("t:nope", 1.0)]), "t:nope"),
            (field(&[("t:wheat", 0.0)]), "weight"),
            (field(&[]), "1 to 4 crops"),
            (
                content::CropField {
                    plot_m: (5.0, 12.0),
                    ..field(&[("t:wheat", 1.0)])
                },
                "6 to 24",
            ),
            (
                content::CropField {
                    path: Some("t:nope".into()),
                    ..field(&[("t:wheat", 1.0)])
                },
                "t:nope",
            ),
            (
                content::CropField {
                    spacing_m: 20.0,
                    ..field(&[("t:wheat", 1.0)])
                },
                "32 to 256",
            ),
        ] {
            let err = resolved_in(vec![], vec![bad], plants())
                .unwrap_err()
                .to_string();
            assert!(err.contains(says), "{says}: {err}");
        }
        let err = resolved_in(vec![], vec![field(&[("t:oak", 1.0)])], plants())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("`field`") && err.contains("crop field 0"),
            "{err}"
        );
    }

    #[test]
    fn more_than_two_crop_fields_are_refused() {
        let fields = vec![field(&[("t:wheat", 1.0)]); 3];
        let err = resolved_in(vec![], fields, plants())
            .unwrap_err()
            .to_string();
        assert!(err.contains("at most 2"), "{err}");
    }

    #[test]
    fn a_feature_group_numbers_in_order_and_freezes_its_region_into_the_biome() {
        let grouped = |plant: &str, group: &str, scale_m, strays| content::SurfaceFeature {
            field: Some(content::FieldDef {
                group: group.into(),
                scale_m,
                strays,
            }),
            ..feature(None, Some(plant), None)
        };
        let (biomes, _, _) = resolved(vec![
            grouped("t:wheat", "flowers", 24.0, 0.1),
            feature(None, Some("t:oak"), None),
            grouped("t:grass", "tufts", 12.0, 0.2),
            grouped("t:kelp", "flowers", 24.0, 0.1),
        ])
        .unwrap();
        let fields: Vec<_> = biomes.as_slice()[0]
            .surface_features
            .as_slice()
            .iter()
            .map(|f| f.field.map(|g| (g.group, g.scale_m, g.strays)))
            .collect();
        assert_eq!(
            fields,
            [
                Some((0, 24.0, 0.1)),
                None,
                Some((1, 12.0, 0.2)),
                Some((0, 24.0, 0.1))
            ]
        );
    }

    #[test]
    fn a_bad_field_names_the_biome_and_feature() {
        let bad = |group: &[(f32, f32)]| {
            resolved(
                group
                    .iter()
                    .map(|&(scale_m, strays)| content::SurfaceFeature {
                        field: Some(content::FieldDef {
                            group: "g".into(),
                            scale_m,
                            strays,
                        }),
                        ..feature(None, Some("t:wheat"), None)
                    })
                    .collect(),
            )
            .unwrap_err()
            .to_string()
        };
        let err = bad(&[(24.0, 0.7)]);
        assert!(
            err.contains("`field`") && err.contains("feature 0") && err.contains("0 to 0.5"),
            "{err}"
        );
        assert!(bad(&[(4.0, 0.1)]).contains("8 to 96"));
        assert!(bad(&[(24.0, 0.1), (32.0, 0.1)]).contains("same scale and strays"));
    }

    #[test]
    fn an_unknown_stage_shape_is_a_load_error_naming_the_plant_and_stage() {
        for (crop, fruit, who, stage) in [
            (Some("t:nope"), None, "wheat", "Stage 1"),
            (None, Some("t:nope"), "oak", "Stage 0"),
        ] {
            let err = resolved_with(vec![], with_stage_shapes(crop, fruit))
                .unwrap_err()
                .to_string();
            assert!(
                err.contains(who) && err.contains(stage) && err.contains("t:nope"),
                "{err}"
            );
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
        let SurfacePlantKind::Stages { materials, len, .. } = features[0].plant.unwrap().kind
        else {
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

    /// The wheat row as a vine crop, with its fruit in the apple Materials.
    fn melon(edit: impl FnOnce(&mut content::FruitVineDef)) -> Vec<content::PlantDef> {
        let mut fruit = content::FruitVineDef {
            stages: vec![stage("t:apple0", Some(32.0)), stage("t:apple1", None)],
            regrow_seconds: 60.0,
            shape: Some("t:apple".into()),
            reach_m: None,
            gap_m: None,
            max_fruit: None,
        };
        edit(&mut fruit);
        vec![row(
            "melon",
            content::PlantKind::Crop(content::CropDef {
                stages: vec![stage("t:wheat0", Some(32.0)), stage("t:wheat1", None)],
                harvest_to: None,
                fruit_vine: Some(fruit),
            }),
        )]
    }

    #[test]
    fn a_vine_crops_fruit_resolves_with_its_defaults_and_freezes_into_the_biome() {
        let (biomes, tables, registry) =
            resolved_with(vec![feature(None, Some("t:melon"), None)], melon(|_| {})).unwrap();
        let Kind::Crop {
            fruit_vine: Some(fruit),
            ..
        } = &tables.plants.plants()[0].kind
        else {
            panic!("a vine crop");
        };
        let apple = registry.id_of(NameKind::Shape, "t:apple").unwrap() as u8;
        assert_eq!(
            (fruit.reach_m, fruit.gap_m, fruit.max_fruit, fruit.shape),
            (1.5, 0.5, 5, Some(apple))
        );
        assert_eq!(fruit.stages[0].material, id(&registry, "t:apple0"));
        let plant = biomes.as_slice()[0].surface_features.as_slice()[0]
            .plant
            .unwrap();
        let SurfacePlantKind::Vine(vine) = plant.kind else {
            panic!("a generated vine");
        };
        assert_eq!(vine.material, id(&registry, "t:wheat1"), "stands ripe");
        assert_eq!(
            (
                vine.len,
                vine.reach_m,
                vine.gap_m,
                vine.max_fruit,
                vine.fruit_shape
            ),
            (2, 1.5, 0.5, 5, apple)
        );
        assert_eq!(
            vine.fruit[..2],
            [id(&registry, "t:apple0"), id(&registry, "t:apple1")]
        );
    }

    #[test]
    fn a_vine_crops_fruit_rules_are_load_errors_naming_the_plant() {
        type Edit = fn(&mut content::FruitVineDef);
        let cases: [(&str, Edit); 5] = [
            ("reach-m", |f| f.reach_m = Some(9.0)),
            ("gap-m", |f| f.gap_m = Some(2.0)),
            ("max-fruit", |f| f.max_fruit = Some(0)),
            ("16 s", |f| f.regrow_seconds = 2.0),
            ("another Plant", |f| {
                f.stages[0].material = "t:wheat0".into()
            }),
        ];
        for (what, edit) in cases {
            let err = resolved_with(vec![], melon(edit)).unwrap_err().to_string();
            assert!(err.contains("melon") && err.contains(what), "{what}: {err}");
        }
    }

    #[test]
    fn a_spreaders_min_neighbours_resolves_and_must_stay_below_its_density() {
        let with = |min: Option<u32>| {
            let mut rows = plants();
            for row in &mut rows {
                if let content::PlantKind::Spreader(s) = &mut row.kind {
                    s.min_neighbours = min;
                }
            }
            resolved_with(vec![], rows)
        };
        let spread = |min| {
            let (_, tables, _) = with(min).unwrap();
            tables
                .plants
                .plants()
                .iter()
                .find_map(|p| match p.kind {
                    Kind::Spreader { min_neighbours, .. } => Some(min_neighbours),
                    _ => None,
                })
                .expect("a spreader")
        };
        assert_eq!((spread(None), spread(Some(3))), (0, 3));
        let err = with(Some(4)).unwrap_err().to_string();
        assert!(
            err.contains("grass") && err.contains("min-neighbours"),
            "{err}"
        );
    }

    #[test]
    fn a_data_mod_names_min_neighbours_and_a_fruit_vine_in_its_rows() {
        let toml = "[[plants]]\nname = \"grass\"\nkind = \"spreader\"\nseed = \"t:g\"\n\
             material = \"t:g\"\nreach-m = 2.0\nmean-seconds = 60.0\ndensity = 5\n\
             min-neighbours = 3\n[plants.needs]\nsoil = [\"t:sand\"]\n\
             [[plants]]\nname = \"melon\"\nkind = \"crop\"\nseed = \"t:m0\"\n\
             stages = [{ material = \"t:m0\", mean-seconds = 20.0 }, { material = \"t:m1\" }]\n\
             fruit-vine = { stages = [{ material = \"t:f0\", mean-seconds = 20.0 }, \
             { material = \"t:f1\" }], reach-m = 2.0, gap-m = 1.0, max-fruit = 3 }\n\
             [plants.needs]\nsoil = [\"t:sand\"]\n";
        let payload = crate::trust::CheckedPayload::for_test(
            "content.toml",
            [("content.toml", toml.as_bytes().to_vec())],
        );
        let declared = crate::data::parse("t", &payload).unwrap();
        let content::PlantKind::Spreader(grass) = &declared.plants[0].kind else {
            panic!("a spreader");
        };
        assert_eq!(grass.min_neighbours, Some(3));
        let content::PlantKind::Crop(melon) = &declared.plants[1].kind else {
            panic!("a crop");
        };
        let fruit = melon.fruit_vine.as_ref().unwrap();
        assert_eq!(
            (
                fruit.reach_m,
                fruit.gap_m,
                fruit.max_fruit,
                fruit.regrow_seconds
            ),
            (Some(2.0), Some(1.0), Some(3), 120.0)
        );
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

    fn branched(edit: impl Fn(&mut content::BranchingDef)) -> Vec<content::PlantDef> {
        let mut rows = plants();
        for row in &mut rows {
            if let content::PlantKind::Tree(tree) = &mut row.kind {
                let mut b = content::BranchingDef {
                    limbs: content::CountRange { min: 3, max: 4 },
                    start: 0.25,
                    reach_m: 3.0,
                    rise: 0.2,
                    fork: Some(0.3),
                    lean_m: 1.0,
                    tiers: None,
                    lobe_radius_m: 1.2,
                };
                edit(&mut b);
                tree.branching = Some(b);
            }
        }
        rows
    }

    #[test]
    fn a_trees_branching_resolves_and_freezes_into_the_biome_and_a_tree_without_it_has_none() {
        let (biomes, tables, _) = resolved_with(
            vec![feature(None, Some("t:oak"), None)],
            branched(|b| b.tiers = Some(true)),
        )
        .unwrap();
        let Kind::Tree(tree) = &tables.plants.plants()[1].kind else {
            panic!("a tree");
        };
        let b = tree.branching.unwrap();
        assert_eq!(
            (b.limbs.min, b.limbs.max, b.fork, b.tiers),
            (3, 4, Some(0.3), true)
        );
        let SurfacePlantKind::Tree(frozen) = biomes.as_slice()[0].surface_features.as_slice()[0]
            .plant
            .unwrap()
            .kind
        else {
            panic!("a tree");
        };
        assert_eq!(frozen.branching, Some(b));
        let (plain, tables, _) =
            resolved_with(vec![feature(None, Some("t:oak"), None)], plants()).unwrap();
        let Kind::Tree(tree) = &tables.plants.plants()[1].kind else {
            panic!("a tree");
        };
        assert_eq!(tree.branching, None);
        let SurfacePlantKind::Tree(frozen) = plain.as_slice()[0].surface_features.as_slice()[0]
            .plant
            .unwrap()
            .kind
        else {
            panic!("a tree");
        };
        assert_eq!(frozen.branching, None);
    }

    #[test]
    fn branching_that_cannot_be_drawn_is_a_load_error_naming_the_tree() {
        let refused = |edit: &dyn Fn(&mut content::BranchingDef)| {
            resolved_with(vec![], branched(edit))
                .unwrap_err()
                .to_string()
        };
        for (what, err) in [
            ("too many limbs", refused(&|b| b.limbs.max = 17)),
            ("a start", refused(&|b| b.start = 0.95)),
            ("a rise", refused(&|b| b.rise = 2.0)),
            ("a fork", refused(&|b| b.fork = Some(0.9))),
            ("past 8 m", refused(&|b| b.reach_m = 7.0)),
            ("no lobe", refused(&|b| b.lobe_radius_m = 0.0)),
            ("a lean past the trunk", refused(&|b| b.lean_m = 9.0)),
        ] {
            assert!(
                err.contains("oak") && err.contains("branching"),
                "{what}: {err}"
            );
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

#[test]
fn a_data_mod_declares_branching_in_its_tree_row() {
    let toml = "[[plants]]\nname = \"pine\"\nkind = \"tree\"\nseed = \"t:sapling\"\n\
         sapling = \"t:sapling\"\ntrunk = \"t:log\"\ncanopy = \"t:leaf\"\n\
         trunk-height-m = [7.0, 10.0]\ncanopy-radius-m = 1.2\nmean-seconds = 60.0\n\
         sapling-yield = [1, 1]\nbranching = { limbs = [3, 5], start = 0.25, reach-m = 3.0, \
         rise = -0.5, lean-m = 0.0, tiers = true, lobe-radius-m = 0.9 }\n\
         [plants.needs]\nsoil = [\"t:sand\"]\n";
    let payload = crate::trust::CheckedPayload::for_test(
        "content.toml",
        [("content.toml", toml.as_bytes().to_vec())],
    );
    let declared = crate::data::parse("t", &payload).unwrap();
    let content::PlantKind::Tree(tree) = &declared.plants[0].kind else {
        panic!("a tree");
    };
    let b = tree.branching.as_ref().unwrap();
    assert_eq!(
        (
            b.limbs.min,
            b.limbs.max,
            b.rise,
            b.fork,
            b.tiers,
            b.lobe_radius_m
        ),
        (3, 5, -0.5, None, Some(true), 0.9)
    );
    let bare = toml.replace("branching", "branchin");
    let payload = crate::trust::CheckedPayload::for_test(
        "content.toml",
        [("content.toml", bare.as_bytes().to_vec())],
    );
    assert!(
        crate::data::parse("t", &payload).is_err(),
        "unknown fields are refused"
    );
}
