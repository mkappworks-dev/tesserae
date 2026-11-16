//! What a worldgen Structure stands at a cell, and the loot its chest there draws: asked by the
//! Server the first time a Container is needed, never by generation, so worldgen stays Voxels.

use super::Worldgen;
use super::fields::hashed;
use super::structures::Structure;
use crate::inventory::Stack;
use crate::loot::LootTable;
use crate::template::Cell;
use crate::voxel::Voxel;

const LOOT_SALT: u64 = 0x1007_c4e5_0000_0001;

/// A Voxel a worldgen Structure stands.
#[derive(Clone, Copy, Debug)]
pub struct Stood<'a> {
    pub voxel: Voxel,
    /// `None` for a Voxel that is not its Template cell's first, or where no table applies.
    pub loot: Option<Loot<'a>>,
}

/// A table and the draw that is this chest's.
#[derive(Clone, Copy, Debug)]
pub struct Loot<'a> {
    pub table: &'a LootTable,
    key: u64,
}

impl Loot<'_> {
    pub fn draw(&self, slots: usize) -> Vec<Option<Stack>> {
        self.table.draw(self.key, slots)
    }
}

impl Worldgen {
    /// This generator answering [`Self::stood_at`] with each Biome's loot, by Biome id.
    pub fn with_loot(mut self, biomes: &[Option<LootTable>]) -> Self {
        self.biome_loot = biomes.into();
        self
    }

    /// The Voxel a worldgen Structure stands at `at`, if one covers it with a Template Voxel, and
    /// the loot that applies: a Settlement building's own table, else the Settlement's; a
    /// wilderness Structure's Biome's at that column.
    ///
    /// Only the first Voxel of a Template cell draws, so a cell sampled by several Voxels holds
    /// one chest's loot; the draw is keyed by the Structure's corner and the cell, in metres and
    /// Template cells, so it is the same at every Voxel size.
    pub fn stood_at(&self, at: [i32; 3]) -> Option<Stood<'_>> {
        let claims = self.claims_near([[at[0], at[2]], [at[0] + 1, at[2] + 1]]);
        for laid in &claims.settlements {
            for (s, rule) in laid.structures.iter().zip(&laid.rules) {
                if !covers(s, at) {
                    continue;
                }
                let settlement = &self.settlement_pass()?.settlements[usize::from(laid.settlement)];
                let table = rule
                    .and_then(|r| settlement.rules[r].loot.as_ref())
                    .or(settlement.loot.as_ref());
                return self.stood_in(s, at, table);
            }
        }
        let s = claims.structures.iter().find(|s| covers(s, at))?;
        let [x_m, z_m] = [at[0], at[2]].map(|g| self.global_m(g) + self.voxel_size / 2.0);
        let table = self
            .biome_at(x_m, z_m)
            .and_then(|b| self.biome_loot.get(b.index()))
            .and_then(Option::as_ref);
        self.stood_in(s, at, table)
    }

    fn stood_in<'a>(
        &'a self,
        s: &Structure,
        at: [i32; 3],
        table: Option<&'a LootTable>,
    ) -> Option<Stood<'a>> {
        let template = &self.templates[usize::from(s.template)];
        let sampled = template.sample(self.voxel_size, s.turn);
        let local = [0, 1, 2].map(|a| at[a] - s.min[a]);
        let Cell::Voxel(voxel) = sampled.cell(local) else {
            return None;
        };
        let source = sampled.source(local);
        let first = (0..3).all(|a| {
            let mut before = local;
            before[a] -= 1;
            local[a] == 0 || sampled.source(before) != source
        });
        let corner_m = [0, 2].map(|a| (s.min[a] as f32 * self.voxel_size).round() as i64 as u64);
        let key = hashed(
            self.seed,
            LOOT_SALT,
            &[
                corner_m[0],
                corner_m[1],
                source[0] as u64,
                source[1] as u64,
                source[2] as u64,
            ],
        );
        let loot = table.filter(|_| first).map(|table| Loot { table, key });
        Some(Stood { voxel, loot })
    }
}

fn covers(s: &Structure, at: [i32; 3]) -> bool {
    (0..3).all(|a| (s.min[a]..s.min[a] + s.size[a]).contains(&at[a]))
}
