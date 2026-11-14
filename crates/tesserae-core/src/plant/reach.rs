//! Where a column, spreader or climber puts its next Voxel: pure functions of
//! a Plant row, a position and the Voxel size, reading the World only through
//! [`Surroundings`].

use super::{
    Direction, FACES, GrowEnv, Growth, Plant, Surroundings, conditions_met, mix, needs_met,
};
use crate::mesh::{FACE_STEPS, NEG_Y, POS_Y};
use crate::voxel::{MaterialId, Voxel};

/// Voxels that span `metres`, rounded up so a cap in metres is never short.
pub(super) fn voxels_in(metres: f32, voxel_size: f32) -> i32 {
    ((metres / voxel_size) - 1e-3).ceil().max(1.0) as i32
}

fn offset(at: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [at[0] + d[0], at[1] + d[1], at[2] + d[2]]
}

/// A hash of the World seed, a position and the Logical step, so a pick is
/// replayable and never advances a generator.
fn pick(seed: u64, at: [i32; 3], step: u64, salt: u64) -> u64 {
    [
        at[0] as u32 as u64,
        at[1] as u32 as u64,
        at[2] as u32 as u64,
        step,
        salt,
    ]
    .into_iter()
    .fold(mix(seed ^ 0x3c6e_f372_fe94_f82b), |h, v| {
        mix(h ^ v.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    })
}

/// Whether `at` holds neither a solid, a fluid nor fire; `None` if unknown.
fn empty(view: &impl Surroundings, at: [i32; 3]) -> Option<bool> {
    view.occupants(at)
        .map(|o| o.solid.is_air() && o.fluid.is_none() && o.burning.is_none())
}

pub(super) struct Column {
    pub max_height_m: f32,
    pub direction: Direction,
    pub stands_in: Option<MaterialId>,
}

/// The Voxel a column adds beyond its end. Only the end Voxel grows, so the
/// rate is one Voxel per `mean-seconds` however tall the run is.
pub(super) fn column(
    plant: &Plant,
    view: &impl Surroundings,
    voxel_size: f32,
    at: [i32; 3],
    voxel: Voxel,
    rule: Column,
) -> Option<Growth> {
    let cap = voxels_in(rule.max_height_m, voxel_size);
    let along = match rule.direction {
        Direction::Up => [0, 1, 0],
        Direction::Down => [0, -1, 0],
    };
    let behind = along.map(|c| -c);
    let material = voxel.material();
    let same = |cell: [i32; 3]| view.occupants(cell).map(|o| o.solid.material() == material);
    let target = offset(at, along);
    if same(target)? {
        return None;
    }
    let mut len = 1;
    let mut cell = offset(at, behind);
    while len < cap && same(cell)? {
        len += 1;
        cell = offset(cell, behind);
    }
    if len >= cap {
        return None;
    }
    let here = view.occupants(target)?;
    let room = here.solid.is_air()
        && here.burning.is_none()
        && match (rule.stands_in, here.fluid) {
            (None, None) => true,
            (Some(fluid), Some(f)) => f.material == fluid,
            _ => false,
        };
    (room && conditions_met(&plant.needs, view, voxel_size, target)).then(|| {
        Growth::one(
            target,
            Voxel::new(material, voxel.shape(), voxel.orientation()),
        )
    })
}

/// A spread target within `reach_m` of `at`, picked by hash uniformly among the
/// Voxels whose centre lies inside that circle, so the chance per square metre
/// of ground does not depend on the Voxel size.
pub(super) fn spread_target(
    seed: u64,
    step: u64,
    at: [i32; 3],
    reach_m: f32,
    voxel_size: f32,
) -> Option<[i32; 3]> {
    let r = (reach_m / voxel_size).max(1.0);
    let span = r.floor() as i32;
    let inside =
        |&(dx, dz): &(i32, i32)| (dx, dz) != (0, 0) && ((dx * dx + dz * dz) as f32) <= r * r;
    let cells = || {
        (-span..=span)
            .flat_map(move |dz| (-span..=span).map(move |dx| (dx, dz)))
            .filter(inside)
    };
    let h = pick(seed, at, step, 1);
    let (dx, dz) = cells().nth((h % cells().count() as u64) as usize)?;
    let dy = ((h >> 40) % 3) as i32 - 1;
    Some([at[0] + dx, at[1] + dy, at[2] + dz])
}

/// Whether fewer than `density` Voxels of `material` lie within `reach_m` of
/// `at`. A cell the view does not know counts as none, so terrain beside an
/// unheld Chunk may overshoot the cap by what it cannot see.
fn under_density(
    view: &impl Surroundings,
    at: [i32; 3],
    material: MaterialId,
    density: u32,
    reach_m: f32,
    voxel_size: f32,
) -> bool {
    let r = (reach_m / voxel_size).max(1.0);
    let span = r.floor() as i32;
    let mut found = 0;
    for dy in -span..=span {
        for dz in -span..=span {
            for dx in -span..=span {
                if ((dx * dx + dy * dy + dz * dz) as f32) > r * r {
                    continue;
                }
                let here = [at[0] + dx, at[1] + dy, at[2] + dz];
                if view
                    .occupants(here)
                    .is_some_and(|o| o.solid.material() == material)
                {
                    found += 1;
                    if found >= density {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// A copy of the Voxel at a Sample-picked target within reach: empty, not under
/// a fluid, on the Plant's soil, with its light, Climate and water, and under
/// the density cap.
#[allow(clippy::too_many_arguments)]
pub(super) fn spread(
    env: &GrowEnv,
    plant: &Plant,
    view: &impl Surroundings,
    step: u64,
    at: [i32; 3],
    voxel: Voxel,
    reach_m: f32,
    density: u32,
) -> Option<Growth> {
    let target = spread_target(env.seed, step, at, reach_m, env.voxel_size)?;
    if !empty(view, target)? || view.occupants(offset(target, [0, 1, 0]))?.fluid.is_some() {
        return None;
    }
    if !needs_met(&plant.needs, view, env.voxel_size, target)
        || !under_density(
            view,
            target,
            voxel.material(),
            density,
            reach_m,
            env.voxel_size,
        )
    {
        return None;
    }
    Some(Growth::one(target, voxel))
}

/// Vine Voxels face-connected to `at`, counted up to `cap`; `None` when the
/// walk meets unknown terrain.
fn group_size(
    view: &impl Surroundings,
    at: [i32; 3],
    material: MaterialId,
    cap: i32,
) -> Option<i32> {
    let mut seen = vec![at];
    let mut next = 0;
    while next < seen.len() && (seen.len() as i32) < cap {
        let from = seen[next];
        next += 1;
        for d in FACES {
            let cell = offset(from, d);
            if seen.contains(&cell) {
                continue;
            }
            if view.occupants(cell)?.solid.material() == material {
                seen.push(cell);
            }
        }
    }
    Some(seen.len() as i32)
}

/// A vine on an adjacent empty Voxel that has the Plant's soil on a face, or
/// below this one, hanging from it, within `length_m` of vine in all.
pub(super) fn climb(
    env: &GrowEnv,
    plant: &Plant,
    view: &impl Surroundings,
    step: u64,
    at: [i32; 3],
    voxel: Voxel,
    length_m: f32,
) -> Option<Growth> {
    let cap = voxels_in(length_m, env.voxel_size);
    if group_size(view, at, voxel.material(), cap)? >= cap {
        return None;
    }
    let h = pick(env.seed, at, step, 2);
    let target = offset(at, FACES[(h % 6) as usize]);
    if !empty(view, target)? || !conditions_met(&plant.needs, view, env.voxel_size, target) {
        return None;
    }
    let held = env.shapes.plane_face(voxel);
    let face = if target[1] < at[1] && held.is_some_and(|f| f != POS_Y && f != NEG_Y) {
        held
    } else {
        attach_face(plant, view, target, h >> 8)
    }?;
    let orientation = env
        .shapes
        .orientation_holding(voxel.material(), voxel.shape(), face)?;
    Some(Growth::one(
        target,
        Voxel::shaped(voxel.material(), voxel.shape(), orientation),
    ))
}

/// A face of `at` whose neighbour is the Plant's soil, starting the search from
/// a hash-picked face. A face the view does not know is not soil.
fn attach_face(plant: &Plant, view: &impl Surroundings, at: [i32; 3], h: u64) -> Option<usize> {
    let first = (h % 6) as usize;
    (0..6).map(|k| (first + k) % 6).find(|&face| {
        view.occupants(offset(at, FACE_STEPS[face]))
            .is_some_and(|o| plant.needs.soil.contains(&o.solid.material()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climate::Climate;
    use crate::light::Light;
    use crate::occupants::{Fluid, Occupants};
    use crate::plant::{Kind, Needs, PlantTable, grow};
    use crate::shape::{Form, NodeBox, Orientation, ShapeDef, ShapeTable};
    use std::collections::{HashMap, HashSet};

    const SOIL: MaterialId = 1;
    const ROCK: MaterialId = 2;
    const WATER: MaterialId = 3;
    const PLANT: MaterialId = 10;
    const SIZES: [f32; 3] = [1.0, 0.5, 0.25];

    #[derive(Default)]
    struct Scene {
        solids: HashMap<[i32; 3], Voxel>,
        fluids: HashSet<[i32; 3]>,
        unknown: HashSet<[i32; 3]>,
        light: u8,
    }

    impl Surroundings for Scene {
        fn occupants(&self, at: [i32; 3]) -> Option<Occupants> {
            (!self.unknown.contains(&at)).then(|| Occupants {
                solid: self.solids.get(&at).copied().unwrap_or(Voxel::AIR),
                fluid: self.fluids.contains(&at).then(|| Fluid::source(WATER)),
                burning: None,
            })
        }

        fn light(&self, at: [i32; 3]) -> Option<Light> {
            (!self.unknown.contains(&at)).then(|| Light::new(self.light, 0))
        }

        fn climate_at(&self, _: f32, _: f32) -> Option<Climate> {
            None
        }
    }

    impl Scene {
        fn lit(light: u8) -> Self {
            Scene {
                light,
                ..Scene::default()
            }
        }

        fn fill(&mut self, from: [i32; 3], to: [i32; 3], voxel: Voxel) {
            for x in from[0]..=to[0] {
                for y in from[1]..=to[1] {
                    for z in from[2]..=to[2] {
                        self.solids.insert([x, y, z], voxel);
                    }
                }
            }
        }

        fn flood(&mut self, from: [i32; 3], to: [i32; 3]) {
            for x in from[0]..=to[0] {
                for y in from[1]..=to[1] {
                    for z in from[2]..=to[2] {
                        self.fluids.insert([x, y, z]);
                    }
                }
            }
        }

        fn apply(&mut self, growth: &Growth) {
            for c in &growth.changes {
                self.solids.insert(c.at, c.voxel);
            }
        }
    }

    fn table(needs: Needs, kind: Kind) -> PlantTable {
        shaped_table(needs, kind, None)
    }

    fn shaped_table(needs: Needs, kind: Kind, shape: Option<crate::voxel::ShapeId>) -> PlantTable {
        let mut table = PlantTable::default();
        table
            .push(Plant {
                name: "p".into(),
                seed: "t:seed".into(),
                first: PLANT,
                shape,
                needs,
                kind,
                sapling_yields: Vec::new(),
            })
            .unwrap();
        table
    }

    fn needs() -> Needs {
        Needs {
            soil: vec![SOIL],
            light_max: 15,
            ..Needs::default()
        }
    }

    fn env<'a>(plants: &'a PlantTable, shapes: &'a ShapeTable, size: f32) -> GrowEnv<'a> {
        GrowEnv {
            plants,
            shapes,
            seed: 0x51ab,
            voxel_size: size,
        }
    }

    fn column_kind(direction: Direction, stands_in: Option<MaterialId>, height: f32) -> Kind {
        Kind::Column {
            max_height_m: height,
            direction,
            mean_seconds: 16.0,
            stands_in,
            yields: Vec::new(),
        }
    }

    /// Grows the column at its end until it stops; returns the run's length.
    fn run_to_the_end(plants: &PlantTable, scene: &mut Scene, size: f32, dir: i32) -> i32 {
        let shapes = ShapeTable::builtin();
        let env = env(plants, &shapes, size);
        let mut end = [0, 0, 0];
        for step in 0..10_000 {
            let Some(g) = grow(&env, scene, step, end, Voxel::cube(PLANT), 0.0) else {
                break;
            };
            assert_eq!(g.changes.len(), 1);
            assert_eq!(g.changes[0].at, [end[0], end[1] + dir, end[2]]);
            scene.apply(&g);
            end = g.changes[0].at;
        }
        (end[1] * dir) + 1
    }

    #[test]
    fn a_column_reaches_its_height_in_metres_at_every_voxel_size() {
        for size in SIZES {
            for (direction, dir) in [(Direction::Up, 1), (Direction::Down, -1)] {
                let plants = table(needs(), column_kind(direction, None, 3.0));
                let mut scene = Scene::lit(15);
                scene.solids.insert([0, -dir, 0], Voxel::cube(SOIL));
                scene.solids.insert([0, 0, 0], Voxel::cube(PLANT));
                let len = run_to_the_end(&plants, &mut scene, size, dir);
                assert_eq!(len as f32 * size, 3.0, "{size} {direction:?}");
            }
        }
    }

    #[test]
    fn a_column_stops_at_an_obstruction_without_dying() {
        for size in SIZES {
            let plants = table(needs(), column_kind(Direction::Up, None, 8.0));
            let mut scene = Scene::lit(15);
            scene.solids.insert([0, -1, 0], Voxel::cube(SOIL));
            scene.solids.insert([0, 0, 0], Voxel::cube(PLANT));
            let roof = (2.0 / size) as i32;
            scene.solids.insert([0, roof, 0], Voxel::cube(ROCK));
            assert_eq!(run_to_the_end(&plants, &mut scene, size, 1), roof, "{size}");
            assert_eq!(scene.solids[&[0, roof - 1, 0]], Voxel::cube(PLANT));
        }
    }

    #[test]
    fn a_column_grows_only_from_its_end_so_the_rate_is_per_voxel() {
        let plants = table(needs(), column_kind(Direction::Up, None, 8.0));
        let shapes = ShapeTable::builtin();
        let mut scene = Scene::lit(15);
        scene.fill([0, 0, 0], [0, 2, 0], Voxel::cube(PLANT));
        let env = env(&plants, &shapes, 1.0);
        for y in 0..2 {
            assert_eq!(
                grow(&env, &scene, 0, [0, y, 0], Voxel::cube(PLANT), 0.0),
                None
            );
        }
        assert!(grow(&env, &scene, 0, [0, 2, 0], Voxel::cube(PLANT), 0.0).is_some());
    }

    #[test]
    fn kelp_grows_through_its_fluid_and_cane_grows_in_air_and_neither_the_other() {
        let shapes = ShapeTable::builtin();
        let kelp = table(needs(), column_kind(Direction::Up, Some(WATER), 8.0));
        let cane = table(needs(), column_kind(Direction::Up, None, 8.0));
        let mut sea = Scene::lit(15);
        sea.solids.insert([0, 0, 0], Voxel::cube(PLANT));
        sea.flood([0, 1, 0], [0, 3, 0]);
        let g = grow(
            &env(&kelp, &shapes, 1.0),
            &sea,
            0,
            [0, 0, 0],
            Voxel::cube(PLANT),
            0.0,
        );
        assert_eq!(g.map(|g| g.changes[0].at), Some([0, 1, 0]));
        assert_eq!(
            grow(
                &env(&cane, &shapes, 1.0),
                &sea,
                0,
                [0, 0, 0],
                Voxel::cube(PLANT),
                0.0
            ),
            None,
            "cane does not grow into water"
        );
        let mut air = Scene::lit(15);
        air.solids.insert([0, 0, 0], Voxel::cube(PLANT));
        assert_eq!(
            grow(
                &env(&kelp, &shapes, 1.0),
                &air,
                0,
                [0, 0, 0],
                Voxel::cube(PLANT),
                0.0
            ),
            None,
            "kelp does not grow in Air"
        );
        assert!(
            grow(
                &env(&cane, &shapes, 1.0),
                &air,
                0,
                [0, 0, 0],
                Voxel::cube(PLANT),
                0.0
            )
            .is_some()
        );
    }

    #[test]
    fn a_column_beside_unknown_terrain_holds() {
        let shapes = ShapeTable::builtin();
        let cane = table(needs(), column_kind(Direction::Up, None, 8.0));
        let mut scene = Scene::lit(15);
        scene.solids.insert([0, 0, 0], Voxel::cube(PLANT));
        scene.unknown.insert([0, 1, 0]);
        assert_eq!(
            grow(
                &env(&cane, &shapes, 1.0),
                &scene,
                0,
                [0, 0, 0],
                Voxel::cube(PLANT),
                0.0
            ),
            None
        );
    }

    fn spreader(reach_m: f32, density: u32, light: (u8, u8)) -> PlantTable {
        table(
            Needs {
                light_min: light.0,
                light_max: light.1,
                ..needs()
            },
            Kind::Spreader {
                reach_m,
                mean_seconds: 16.0,
                density,
                yields: Vec::new(),
            },
        )
    }

    fn targets(plants: &PlantTable, scene: &Scene, size: f32, steps: u64) -> Vec<[i32; 3]> {
        let shapes = ShapeTable::builtin();
        let env = env(plants, &shapes, size);
        (0..steps)
            .filter_map(|step| grow(&env, scene, step, [0, 0, 0], Voxel::cube(PLANT), 0.0))
            .map(|g| g.changes[0].at)
            .collect()
    }

    fn meadow(size: f32, reach_m: f32) -> Scene {
        let span = (reach_m / size) as i32 + 2;
        let mut scene = Scene::lit(10);
        scene.fill([-span, -1, -span], [span, -1, span], Voxel::cube(SOIL));
        scene.solids.insert([0, 0, 0], Voxel::cube(PLANT));
        scene
    }

    #[test]
    fn a_spreader_reaches_exactly_its_metres_at_every_voxel_size() {
        for size in SIZES {
            let plants = spreader(2.0, 1000, (0, 15));
            let scene = meadow(size, 2.0);
            let hits = targets(&plants, &scene, size, 4000);
            assert!(!hits.is_empty(), "{size}");
            let far = hits
                .iter()
                .map(|t| ((t[0] * t[0] + t[2] * t[2]) as f32).sqrt() * size)
                .fold(0.0, f32::max);
            assert!(far <= 2.0 + 1e-4, "{size}: spread {far} m past reach");
            assert!(far >= 1.5, "{size}: only reached {far} m");
        }
    }

    #[test]
    fn a_spreader_lands_as_often_per_sample_at_every_voxel_size() {
        let plants = spreader(2.0, 1000, (0, 15));
        for size in SIZES {
            let scene = meadow(size, 2.0);
            let samples = 6000;
            let landed = targets(&plants, &scene, size, samples).len() as f32 / samples as f32;
            assert!(
                (landed - 1.0 / 3.0).abs() < 0.03,
                "{size}: {landed} of Samples landed on a flat meadow"
            );
        }
    }

    #[test]
    fn a_spreader_picks_the_same_targets_for_the_same_inputs() {
        let plants = spreader(2.0, 1000, (0, 15));
        let scene = meadow(1.0, 2.0);
        assert_eq!(
            targets(&plants, &scene, 1.0, 500),
            targets(&plants, &scene, 1.0, 500)
        );
    }

    #[test]
    fn a_spreader_never_lands_on_non_soil_occupied_or_flooded_ground() {
        let plants = spreader(2.0, 1000, (0, 15));
        let mut scene = meadow(1.0, 2.0);
        scene.fill([1, -1, -4], [4, -1, 4], Voxel::cube(ROCK));
        scene.fill([-4, 0, 1], [-1, 0, 4], Voxel::cube(ROCK));
        scene.flood([-4, 0, -4], [-1, 1, -1]);
        let hits = targets(&plants, &scene, 1.0, 8000);
        assert!(hits.iter().any(|t| t[0] <= 0 && t[2] <= 0 || t[2] >= 0));
        for t in hits {
            assert_ne!(
                scene
                    .solids
                    .get(&[t[0], t[1] - 1, t[2]])
                    .map(|v| v.material()),
                Some(ROCK),
                "{t:?} is on rock"
            );
            assert!(!scene.solids.contains_key(&t), "{t:?} is occupied");
            assert!(!scene.fluids.contains(&t), "{t:?} is in a fluid");
            assert!(
                !scene.fluids.contains(&[t[0], t[1] + 1, t[2]]),
                "{t:?} is under a fluid"
            );
        }
    }

    /// Spreads a field from one Voxel for `steps` Samples at density `cap`,
    /// checking each landing against the cap as it goes.
    fn fill_field(size: f32, cap: u32, steps: u64) -> usize {
        let reach = 2.0;
        let plants = spreader(reach, cap, (0, 15));
        let shapes = ShapeTable::builtin();
        let env = env(&plants, &shapes, size);
        let mut scene = meadow(size, reach);
        let mut sources = vec![[0, 0, 0]];
        for step in 0..steps {
            let from = sources[step as usize % sources.len()];
            let Some(g) = grow(&env, &scene, step, from, Voxel::cube(PLANT), 0.0) else {
                continue;
            };
            let at = g.changes[0].at;
            let near = scene
                .solids
                .iter()
                .filter(|(c, v)| {
                    v.material() == PLANT && {
                        let d: [i32; 3] = std::array::from_fn(|i| c[i] - at[i]);
                        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) as f32 <= (reach / size).powi(2)
                    }
                })
                .count();
            assert!(near < cap as usize, "{size}: {near} within reach of {at:?}");
            scene.apply(&g);
            sources.push(at);
        }
        sources.len()
    }

    #[test]
    fn the_density_cap_holds_a_field_to_its_count() {
        for size in SIZES {
            let capped = fill_field(size, 3, 6000);
            let free = fill_field(size, 1000, 6000);
            assert!(capped >= 3, "{size}: filled to {capped}");
            assert!(
                capped * 2 < free,
                "{size}: a cap of 3 held {capped} against {free} uncapped"
            );
        }
    }

    #[test]
    fn mushrooms_spread_in_the_dark_and_grass_in_the_light() {
        let mushroom = spreader(2.0, 1000, (0, 4));
        let grass = spreader(2.0, 1000, (8, 15));
        for (light, mushrooms, grasses) in [(2, true, false), (12, false, true), (6, false, false)]
        {
            let mut scene = meadow(1.0, 2.0);
            scene.light = light;
            assert_eq!(
                !targets(&mushroom, &scene, 1.0, 500).is_empty(),
                mushrooms,
                "mushroom at {light}"
            );
            assert_eq!(
                !targets(&grass, &scene, 1.0, 500).is_empty(),
                grasses,
                "grass at {light}"
            );
        }
    }

    #[test]
    fn a_spreader_beside_unknown_terrain_never_reads_it_as_air() {
        let plants = spreader(2.0, 1000, (0, 15));
        let mut scene = meadow(1.0, 2.0);
        for x in 1..=3 {
            for y in -2..=2 {
                for z in -3..=3 {
                    scene.unknown.insert([x, y, z]);
                }
            }
        }
        for t in targets(&plants, &scene, 1.0, 4000) {
            assert!(t[0] <= 0, "{t:?} is in unknown terrain");
        }
    }

    fn vines() -> (ShapeTable, crate::voxel::ShapeId) {
        let mut shapes = ShapeTable::builtin();
        let id = shapes
            .register(ShapeDef {
                form: Form::Plane,
                boxes: vec![NodeBox::new([1, 1, 0], [15, 15, 2])],
                ..ShapeDef::default()
            })
            .unwrap();
        (shapes, id)
    }

    #[test]
    fn the_orientation_that_holds_a_face_puts_the_quad_against_it_for_all_six() {
        let (shapes, id) = vines();
        for face in 0..6 {
            let o = shapes
                .orientation_holding(PLANT, id, face)
                .unwrap_or_else(|| panic!("no Orientation holds face {face}"));
            let vine = Voxel::shaped(PLANT, id, o);
            assert_eq!(shapes.plane_face(vine), Some(face));
            let b = shapes.selection(vine, crate::shape::Connections::NONE)[0];
            let axis = face / 2;
            let (near, far) = if face % 2 == 0 {
                (b.min[axis], b.max[axis])
            } else {
                (16 - b.max[axis], 16 - b.min[axis])
            };
            assert!(near < 2 && far <= 2, "face {face}: {b:?}");
        }
        assert_eq!(
            shapes.plane_face(Voxel::shaped(PLANT, id, Orientation::IDENTITY)),
            Some(crate::mesh::NEG_Z)
        );
        assert_eq!(shapes.plane_face(Voxel::cube(PLANT)), None);
        assert_eq!(shapes.orientation_holding(PLANT, 0, 0), None);
    }

    #[test]
    fn a_column_and_a_spreader_grow_in_the_shape_they_were_placed_in() {
        const SHAPE: crate::voxel::ShapeId = 9;
        let shapes = ShapeTable::builtin();
        let cane = Voxel::new(PLANT, SHAPE, 0);
        let column = table(needs(), column_kind(Direction::Up, None, 4.0));
        let mut scene = Scene::lit(15);
        scene.solids.insert([0, -1, 0], Voxel::cube(SOIL));
        scene.solids.insert([0, 0, 0], cane);
        let g = grow(&env(&column, &shapes, 1.0), &scene, 0, [0, 0, 0], cane, 0.0).unwrap();
        assert_eq!(g.changes[0].voxel, cane);
        let grass = spreader(2.0, 1000, (0, 15));
        let mut scene = meadow(1.0, 2.0);
        scene.solids.insert([0, 0, 0], cane);
        let env = env(&grass, &shapes, 1.0);
        let spread: Vec<_> = (0..200)
            .filter_map(|step| grow(&env, &scene, step, [0, 0, 0], cane, 0.0))
            .collect();
        assert!(!spread.is_empty());
        assert!(spread.iter().all(|g| g.changes[0].voxel == cane));
    }

    #[test]
    fn placing_a_seed_uses_the_plants_shape_and_ignores_the_players() {
        let (mut shapes, vine) = vines();
        let cross = shapes
            .register(ShapeDef {
                form: Form::Cross,
                boxes: vec![NodeBox::new([4, 0, 4], [12, 12, 12])],
                ..ShapeDef::default()
            })
            .unwrap();
        let chosen = Voxel::shaped(PLANT, 1, Orientation::all().nth(5).unwrap());
        let wheat = shaped_table(needs(), column_kind(Direction::Up, None, 1.0), Some(cross));
        for face in 0..6 {
            assert_eq!(
                wheat.placed(&shapes, chosen, face),
                Voxel::new(PLANT, cross, 0),
                "face {face}"
            );
        }
        let bare = table(needs(), column_kind(Direction::Up, None, 1.0));
        assert_eq!(bare.placed(&shapes, chosen, 3), chosen);
        let other = Voxel::shaped(ROCK, 1, Orientation::IDENTITY);
        assert_eq!(wheat.placed(&shapes, other, 3), other);
        assert_eq!(wheat.fixed_shape(PLANT), Some(cross));
        assert_eq!(bare.fixed_shape(PLANT), None);
        let ivy = shaped_table(
            needs(),
            Kind::Climber {
                mean_seconds: 16.0,
                length_m: 4.0,
                yields: Vec::new(),
            },
            Some(vine),
        );
        for face in 0..6 {
            let put = ivy.placed(&shapes, chosen, face);
            assert_eq!(put.shape(), vine);
            assert_eq!(
                shapes.plane_face(put),
                Some(face ^ 1),
                "struck face {face} holds the Voxel struck"
            );
        }
    }

    fn climber(length_m: f32) -> PlantTable {
        table(
            needs(),
            Kind::Climber {
                mean_seconds: 16.0,
                length_m,
                yields: Vec::new(),
            },
        )
    }

    fn held(shapes: &ShapeTable, id: crate::voxel::ShapeId, face: usize) -> Voxel {
        Voxel::shaped(
            PLANT,
            id,
            shapes.orientation_holding(PLANT, id, face).unwrap(),
        )
    }

    /// Grows every vine Sample repeatedly over a scene; returns the vine cells.
    fn spread_vines(
        plants: &PlantTable,
        shapes: &ShapeTable,
        scene: &mut Scene,
        size: f32,
        rounds: u64,
    ) -> Vec<[i32; 3]> {
        let env = env(plants, shapes, size);
        for step in 0..rounds {
            let cells: Vec<([i32; 3], Voxel)> = scene
                .solids
                .iter()
                .filter(|(_, v)| v.material() == PLANT)
                .map(|(&c, &v)| (c, v))
                .collect();
            let mut cells = cells;
            cells.sort_by_key(|(c, _)| *c);
            for (at, voxel) in cells {
                if let Some(g) = grow(&env, scene, step, at, voxel, 0.0) {
                    scene.apply(&g);
                }
            }
        }
        let mut vines: Vec<_> = scene
            .solids
            .iter()
            .filter(|(_, v)| v.material() == PLANT)
            .map(|(&c, _)| c)
            .collect();
        vines.sort();
        vines
    }

    #[test]
    fn a_vine_spreads_along_a_wall_to_its_length_in_metres_and_never_through_a_solid() {
        let (shapes, id) = vines();
        for size in SIZES {
            let plants = climber(2.0);
            let mut scene = Scene::lit(10);
            let wall = (6.0 / size) as i32;
            scene.fill([1, -2, -wall], [3, wall, wall], Voxel::cube(SOIL));
            scene
                .solids
                .insert([0, 0, 0], held(&shapes, id, crate::mesh::POS_X));
            let vines = spread_vines(&plants, &shapes, &mut scene, size, 600);
            assert_eq!(
                vines.len() as f32 * size,
                2.0,
                "{size}: {} vines",
                vines.len()
            );
            for v in &vines {
                let o = scene.solids[v];
                let face = shapes.plane_face(o).unwrap();
                let step = crate::mesh::FACE_STEPS[face];
                let behind = [v[0] + step[0], v[1] + step[1], v[2] + step[2]];
                let held_solid = scene.solids.get(&behind).map(|s| s.material());
                let hangs = scene
                    .solids
                    .get(&[v[0], v[1] + 1, v[2]])
                    .is_some_and(|s| s.material() == PLANT);
                assert!(
                    held_solid == Some(SOIL) || hangs,
                    "{v:?} holds face {face} with nothing there"
                );
                assert!(v[0] <= 0, "{v:?} is inside the wall");
            }
        }
    }

    #[test]
    fn a_vine_hangs_below_the_last_vine_and_below_an_overhang() {
        let (shapes, id) = vines();
        let plants = climber(8.0);
        let mut scene = Scene::lit(10);
        scene.fill([1, 0, 0], [1, 0, 0], Voxel::cube(SOIL));
        scene
            .solids
            .insert([0, 0, 0], held(&shapes, id, crate::mesh::POS_X));
        let vines = spread_vines(&plants, &shapes, &mut scene, 1.0, 400);
        assert!(
            vines.contains(&[0, -3, 0]),
            "a vine hangs past the wall's end: {vines:?}"
        );
        for v in vines.iter().filter(|v| v[0] == 0 && v[2] == 0 && v[1] < 0) {
            assert_eq!(
                shapes.plane_face(scene.solids[v]),
                Some(crate::mesh::POS_X),
                "a hanging vine keeps the face above"
            );
        }

        let mut ceiling = Scene::lit(10);
        ceiling.fill([-3, 1, -3], [3, 1, 3], Voxel::cube(SOIL));
        ceiling
            .solids
            .insert([0, 0, 0], held(&shapes, id, crate::mesh::POS_Y));
        let vines = spread_vines(&plants, &shapes, &mut ceiling, 1.0, 300);
        assert!(vines.len() > 3, "{vines:?}");
        assert!(vines.iter().all(|v| v[1] == 0), "{vines:?}");
        assert!(
            vines
                .iter()
                .all(|v| shapes.plane_face(ceiling.solids[v]) == Some(crate::mesh::POS_Y))
        );
    }

    #[test]
    fn a_vine_is_held_by_the_face_it_names_or_the_vine_above() {
        let (shapes, id) = vines();
        let plants = climber(8.0);
        let at = [4, 4, 4];
        let vine = held(&shapes, id, crate::mesh::POS_X);
        let scene = |cells: &[([i32; 3], MaterialId)]| {
            let cells = cells.to_vec();
            move |c: [i32; 3]| {
                Some(Occupants {
                    solid: cells
                        .iter()
                        .find(|(p, _)| *p == c)
                        .map_or(Voxel::AIR, |&(_, m)| Voxel::cube(m)),
                    fluid: None,
                    burning: None,
                })
            }
        };
        let lacks = |cells: &[([i32; 3], MaterialId)]| {
            plants
                .lacks_support(&shapes, vine, at, &scene(cells))
                .is_some()
        };
        assert!(!lacks(&[([5, 4, 4], SOIL)]), "soil on the held face");
        assert!(lacks(&[([3, 4, 4], SOIL)]), "soil on the far face only");
        assert!(lacks(&[([4, 3, 4], SOIL)]), "soil below only");
        assert!(!lacks(&[([4, 5, 4], PLANT)]), "hanging from a vine");
        assert!(lacks(&[]));
        assert!(
            plants.lacks_support(&shapes, vine, at, &|_| None).is_none(),
            "unheld terrain is never read as Air"
        );
        let generic = |cells: &[([i32; 3], MaterialId)]| {
            plants.lacks_soil(PLANT, at, &scene(cells)).is_some()
        };
        assert!(
            !generic(&[([4, 5, 4], PLANT)]),
            "any climber hangs from one above"
        );
    }
}
