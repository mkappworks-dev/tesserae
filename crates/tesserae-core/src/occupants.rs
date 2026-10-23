//! What is at a Voxel's position beyond its packed solid: a fluid and a fire.
//!
//! A Voxel stays `(Material, Shape, Orientation)`. Fluid and combustion state
//! is an [`Overlay`] stored beside it, and [`Occupants`] is the one reading of
//! the pair that generation, Edits, light, meshing and LOD share.

use std::borrow::Cow;

use crate::chunk::{CHUNK_SIZE, Chunk, Palette, index};
use crate::fluid::FluidTable;
use crate::shape::{Connections, MAX_VOIDS};
use crate::tables::Tables;
use crate::voxel::{MaterialId, Voxel};

/// Flowing fluid's state in one void of a Voxel.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Level {
    /// Surface height as sixteenths of the Voxel edge, `1..=FULL_SURFACE`, and
    /// 0 for a dry void. A fraction of the Voxel and not a distance, so it does
    /// not scale with Voxel size.
    pub surface: u8,
    /// Sideways steps left before the fluid stops spreading: its definition's
    /// metres of reach, quantized to this World's Voxels by
    /// [`crate::flow::FlowScale`].
    pub reach: u16,
    /// Gravity descent since reach was last renewed, in sixteenths of a Voxel.
    pub descent: u16,
}

impl Level {
    pub const DRY: Level = Level {
        surface: 0,
        reach: 0,
        descent: 0,
    };

    pub const fn is_wet(self) -> bool {
        self.surface > 0
    }
}

/// Fluid occupying a Voxel's empty space: the whole of it in air, or the
/// voids ([`crate::shape::Void`]) a partial Shape leaves.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Fluid {
    pub material: MaterialId,
    /// A source persists until an Edit or a reaction removes it, and fills
    /// every void; flowing fluid depends on one and recedes without it.
    pub source: bool,
    /// The first void's [`Level`], flattened: in air the only one. Full and
    /// carrying no flow state for a source.
    pub surface: u8,
    pub reach: u16,
    pub descent: u16,
    /// The [`Level`] in each further void, dry where a Shape has fewer. Kept
    /// apart because a Shape's separate voids fill separately: one shared
    /// state would let a wet void lend its reach to water across a partition.
    pub others: [Level; MAX_VOIDS - 1],
}

impl Fluid {
    pub const FULL_SURFACE: u8 = 16;

    pub const fn source(material: MaterialId) -> Self {
        Fluid {
            material,
            source: true,
            surface: Self::FULL_SURFACE,
            reach: 0,
            descent: 0,
            others: [Level::DRY; MAX_VOIDS - 1],
        }
    }

    /// Flowing fluid with `levels` by void, or `None` if every one is dry.
    pub fn flowing(material: MaterialId, levels: [Level; MAX_VOIDS]) -> Option<Self> {
        levels.iter().any(|l| l.is_wet()).then(|| Fluid {
            material,
            source: false,
            surface: levels[0].surface,
            reach: levels[0].reach,
            descent: levels[0].descent,
            others: [levels[1], levels[2], levels[3]],
        })
    }

    /// What stands in `void`: full for a source, which fills them all.
    pub fn level(&self, void: usize) -> Level {
        match void {
            0 => Level {
                surface: self.surface,
                reach: self.reach,
                descent: self.descent,
            },
            _ if self.source => Level {
                surface: Self::FULL_SURFACE,
                ..Level::DRY
            },
            i => self.others.get(i - 1).copied().unwrap_or(Level::DRY),
        }
    }

    pub fn levels(&self) -> [Level; MAX_VOIDS] {
        std::array::from_fn(|i| self.level(i))
    }
}

/// Whole-Voxel fuel progress of a burning solid.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Burning {
    /// Fuel left, in the simulation's logical time quantum. The quantum is
    /// fixed when combustion is simulated; until then this only has to be
    /// carried, compared and cleared.
    pub fuel: u16,
}

/// Everything stored beside a packed Voxel. `NONE` for dry, unburning terrain.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Overlay {
    pub fluid: Option<Fluid>,
    pub burning: Option<Burning>,
}

impl Overlay {
    pub const NONE: Overlay = Overlay {
        fluid: None,
        burning: None,
    };
}

/// One Chunk's overlays, beside its Voxels and compressed the same way, so a
/// dry Chunk and a uniform lake both hold one word and no heap.
#[derive(Clone, Debug)]
pub struct ChunkOverlay {
    storage: Palette<Overlay>,
}

impl Default for ChunkOverlay {
    fn default() -> Self {
        Self::none()
    }
}

impl ChunkOverlay {
    pub const fn none() -> Self {
        ChunkOverlay {
            storage: Palette::Uniform(Overlay::NONE),
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Overlay {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        self.storage.get(index(x, y, z))
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, overlay: Overlay) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        self.storage.set(index(x, y, z), overlay);
    }

    pub fn uniform(&self) -> Option<Overlay> {
        self.storage.uniform()
    }

    /// From the palette alone, so it errs towards `true` as
    /// [`crate::Chunk::may_contain_shaped`] does: the direction a gate needs.
    pub fn may_contain_fluid(&self) -> bool {
        self.storage.kinds().iter().any(|o| o.fluid.is_some())
    }

    pub fn may_contain_burning(&self) -> bool {
        self.storage.kinds().iter().any(|o| o.burning.is_some())
    }

    pub fn heap_bytes(&self) -> usize {
        self.storage.heap_bytes()
    }

    /// Every fluid Material the palette holds, possibly with repeats.
    pub fn fluids(&self) -> impl Iterator<Item = MaterialId> + '_ {
        self.storage
            .kinds()
            .iter()
            .filter_map(|o| o.fluid.map(|f| f.material))
    }
}

/// The Voxel light treats a stored pair as: fluid in empty space dims and
/// emits as a cube of its Material, whatever its surface height, since a Voxel
/// holds one light value. Fluid beside a solid keeps the solid here; light
/// combines the two through [`Tables::wet_light_params`].
#[inline]
pub fn optical(voxel: Voxel, overlay: Overlay) -> Voxel {
    match overlay.fluid {
        Some(fluid) if voxel.is_air() => Voxel::cube(fluid.material),
        _ => voxel,
    }
}

/// [`optical`] for a whole Chunk, copied only when its overlay holds fluid.
pub fn optical_chunk<'a>(chunk: &'a Chunk, overlay: &ChunkOverlay) -> Cow<'a, Chunk> {
    if !overlay.may_contain_fluid() {
        return Cow::Borrowed(chunk);
    }
    let mut seen = chunk.clone();
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let voxel = chunk.get(x, y, z);
                let lit = optical(voxel, overlay.get(x, y, z));
                if lit != voxel {
                    seen.set(x, y, z, lit);
                }
            }
        }
    }
    Cow::Owned(seen)
}

/// The canonical reading of one position: its solid, its fluid and its fire.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Occupants {
    /// Air when nothing solid is there, whatever fluid is.
    pub solid: Voxel,
    pub fluid: Option<Fluid>,
    pub burning: Option<Burning>,
}

impl Occupants {
    /// The one reading of a stored pair. A packed fluid Voxel is a full source
    /// with nothing solid, which is what generation and a legacy placement
    /// make; every other fluid state is in the overlay.
    pub fn of(voxel: Voxel, overlay: Overlay, fluids: &FluidTable) -> Self {
        if fluids.contains(voxel.material()) {
            debug_assert!(
                overlay.fluid.is_none(),
                "a packed fluid carries no overlay fluid"
            );
            return Occupants {
                solid: Voxel::AIR,
                fluid: Some(Fluid::source(voxel.material())),
                burning: overlay.burning,
            };
        }
        Occupants {
            solid: voxel,
            fluid: overlay.fluid,
            burning: overlay.burning,
        }
    }

    /// What a whole-Voxel placement of `voxel` leaves here: the one rule for a
    /// player's click, a Mod's Command and a `dev` builder.
    ///
    /// A fluid Material placed this way is a source. Air breaks the solid and
    /// keeps the fluid; where there was no solid it empties the Voxel, as it
    /// always did. A solid that leaves voids keeps the fluid in them and one
    /// that leaves none displaces it. A source stays a source and flow stays
    /// flow, each void taking the best level of the old voids it overlaps and
    /// staying dry where the new solid now fills what was wet. Any change to
    /// the solid ends its fire.
    pub fn place_solid(self, voxel: Voxel, tables: &Tables) -> Self {
        if tables.fluids.contains(voxel.material()) {
            return Occupants {
                solid: Voxel::AIR,
                fluid: Some(Fluid::source(voxel.material())),
                burning: None,
            };
        }
        let fluid = match self.fluid {
            Some(_) if voxel.is_air() && self.solid.is_air() => None,
            Some(fluid) => carry(fluid, self.solid, voxel, tables),
            None => None,
        };
        Occupants {
            solid: voxel,
            fluid,
            burning: None,
        }
    }

    /// The fluid Material here, wherever it stands.
    pub fn fluid_material(&self) -> Option<MaterialId> {
        self.fluid.map(|f| f.material)
    }

    /// The stored form of this reading, the inverse of [`Occupants::of`].
    pub fn encode(self, fluids: &FluidTable) -> (Voxel, Overlay) {
        match self.fluid {
            Some(fluid)
                if self.solid.is_air()
                    && fluid == Fluid::source(fluid.material)
                    && fluids.contains(fluid.material) =>
            {
                (
                    Voxel::cube(fluid.material),
                    Overlay {
                        fluid: None,
                        burning: self.burning,
                    },
                )
            }
            fluid => (
                self.solid,
                Overlay {
                    fluid,
                    burning: self.burning,
                },
            ),
        }
    }
}

/// `fluid` in `from`'s voids moved into `to`'s. Read without arms: a connected
/// Shape's flow is derived again from its actual arms once it is evaluated.
fn carry(fluid: Fluid, from: Voxel, to: Voxel, tables: &Tables) -> Option<Fluid> {
    let shapes = &tables.shapes;
    if shapes.void_slots(to) == 0 {
        return None;
    }
    if fluid.source {
        return Some(fluid);
    }
    let old = shapes.voids(from, Connections::NONE);
    let new = shapes.voids(to, Connections::NONE);
    let mut levels = [Level::DRY; MAX_VOIDS];
    for (j, void) in new.iter().enumerate() {
        let best = old
            .iter()
            .enumerate()
            .filter(|(_, o)| o.overlaps(void))
            .map(|(i, _)| fluid.level(i))
            .filter(|l| l.surface > void.floor)
            .max_by_key(|l| (l.reach, l.descent, l.surface));
        if let Some(level) = best {
            levels[j] = level;
        }
    }
    Fluid::flowing(fluid.material, levels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluid::FluidTable;
    use crate::material::{STONE, WATER};

    #[test]
    fn a_packed_fluid_voxel_is_a_full_source_with_no_solid() {
        let fluids = FluidTable::builtin();
        let got = Occupants::of(Voxel::cube(WATER), Overlay::NONE, &fluids);
        assert_eq!(
            got,
            Occupants {
                solid: Voxel::AIR,
                fluid: Some(Fluid::source(WATER)),
                burning: None,
            }
        );
        let dry = Occupants::of(Voxel::cube(STONE), Overlay::NONE, &fluids);
        assert_eq!(dry.solid, Voxel::cube(STONE));
        assert_eq!((dry.fluid, dry.burning), (None, None));
    }

    fn stair() -> Voxel {
        Voxel::new(STONE, crate::SHAPE_STAIR, 0)
    }

    /// The fluid Material has one home: the packed Voxel when it is a full
    /// source with no solid, the overlay otherwise, and never both.
    #[test]
    fn the_encoding_is_canonical_and_round_trips() {
        let fluids = FluidTable::builtin();
        let flowing = Fluid {
            source: false,
            surface: 9,
            reach: 3,
            descent: 8,
            ..Fluid::source(WATER)
        };
        let cases = [
            (
                Occupants {
                    solid: Voxel::AIR,
                    fluid: Some(Fluid::source(WATER)),
                    burning: None,
                },
                (Voxel::cube(WATER), Overlay::NONE),
            ),
            (
                Occupants {
                    solid: Voxel::AIR,
                    fluid: Some(flowing),
                    burning: None,
                },
                (
                    Voxel::AIR,
                    Overlay {
                        fluid: Some(flowing),
                        burning: None,
                    },
                ),
            ),
            (
                Occupants {
                    solid: stair(),
                    fluid: Some(Fluid::source(WATER)),
                    burning: Some(Burning { fuel: 3 }),
                },
                (
                    stair(),
                    Overlay {
                        fluid: Some(Fluid::source(WATER)),
                        burning: Some(Burning { fuel: 3 }),
                    },
                ),
            ),
            (
                Occupants {
                    solid: Voxel::AIR,
                    fluid: None,
                    burning: None,
                },
                (Voxel::AIR, Overlay::NONE),
            ),
        ];
        for (occupants, stored) in cases {
            assert_eq!(occupants.encode(&fluids), stored, "{occupants:?}");
            let (voxel, overlay) = stored;
            assert_eq!(Occupants::of(voxel, overlay, &fluids), occupants);
        }
    }

    fn wet(solid: Voxel, fluid: Fluid, fuel: Option<u16>) -> Occupants {
        Occupants {
            solid,
            fluid: Some(fluid),
            burning: fuel.map(|fuel| Burning { fuel }),
        }
    }

    /// A full solid displaces fluid, a partial one keeps it in the space it
    /// leaves open, breaking only the solid keeps it, a fluid Material placed
    /// the old way is a source, and a solid change ends whatever was burning. A
    /// placement over a Voxel with no solid keeps its whole-Voxel meaning, so
    /// air over water still empties it. Source and flow keep their identity.
    #[test]
    fn placing_a_solid_follows_the_layer_rules() {
        let tables = Tables::builtin();
        let water = Fluid::source(WATER);
        let deep = Fluid {
            source: false,
            surface: 12,
            reach: 3,
            ..water
        };
        let slab = Voxel::new(STONE, crate::SHAPE_SLAB, 0);
        let flowing = Fluid {
            source: false,
            surface: 4,
            ..water
        };
        let empty = Occupants {
            solid: Voxel::AIR,
            fluid: None,
            burning: None,
        };
        let cases = [
            (
                "stone into water",
                wet(Voxel::AIR, water, None),
                Voxel::cube(STONE),
                Occupants {
                    solid: Voxel::cube(STONE),
                    ..empty
                },
            ),
            (
                "air over water",
                wet(Voxel::AIR, water, None),
                Voxel::AIR,
                empty,
            ),
            (
                "water onto burning stone",
                Occupants {
                    solid: Voxel::cube(STONE),
                    fluid: None,
                    burning: Some(Burning { fuel: 5 }),
                },
                Voxel::cube(WATER),
                wet(Voxel::AIR, water, None),
            ),
            (
                "water onto flowing water",
                wet(Voxel::AIR, flowing, None),
                Voxel::cube(WATER),
                wet(Voxel::AIR, water, None),
            ),
            (
                "break a waterlogged stair",
                wet(stair(), flowing, Some(2)),
                Voxel::AIR,
                wet(Voxel::AIR, flowing, None),
            ),
            (
                "stone for a burning stair",
                Occupants {
                    solid: stair(),
                    fluid: None,
                    burning: Some(Burning { fuel: 5 }),
                },
                Voxel::cube(STONE),
                Occupants {
                    solid: Voxel::cube(STONE),
                    ..empty
                },
            ),
            (
                "stair into a source",
                wet(Voxel::AIR, water, None),
                stair(),
                wet(stair(), water, None),
            ),
            (
                "stair into deep flow",
                wet(Voxel::AIR, deep, None),
                stair(),
                wet(stair(), deep, None),
            ),
            (
                "slab into flow shallower than the slab",
                wet(Voxel::AIR, flowing, None),
                slab,
                Occupants {
                    solid: slab,
                    ..empty
                },
            ),
            (
                "slab for a waterlogged stair",
                wet(stair(), water, Some(4)),
                slab,
                wet(slab, water, None),
            ),
            (
                "stone into a waterlogged stair",
                wet(stair(), water, None),
                Voxel::cube(STONE),
                Occupants {
                    solid: Voxel::cube(STONE),
                    ..empty
                },
            ),
        ];
        for (name, before, voxel, after) in cases {
            assert_eq!(before.place_solid(voxel, &tables), after, "{name}");
        }
    }

    /// Dry terrain and a uniform lake allocate nothing for the overlay, and one
    /// wet Voxel costs a 1-bit index plane.
    #[test]
    fn dry_and_uniform_lake_chunks_allocate_no_overlay() {
        let dry = ChunkOverlay::none();
        assert_eq!(dry.uniform(), Some(Overlay::NONE));
        assert_eq!(dry.heap_bytes(), 0);
        assert!(!dry.may_contain_fluid() && !dry.may_contain_burning());

        let lake = crate::Chunk::filled(Voxel::cube(WATER));
        assert_eq!(lake.heap_bytes() + ChunkOverlay::none().heap_bytes(), 0);

        let mut wet = ChunkOverlay::none();
        let flowing = Overlay {
            fluid: Some(Fluid {
                source: false,
                surface: 4,
                reach: 2,
                ..Fluid::source(WATER)
            }),
            burning: None,
        };
        wet.set(3, 4, 5, flowing);
        assert_eq!(wet.get(3, 4, 5), flowing);
        assert_eq!(wet.get(3, 4, 6), Overlay::NONE);
        assert_eq!(wet.uniform(), None);
        assert!(wet.may_contain_fluid() && !wet.may_contain_burning());
        assert_eq!(
            wet.heap_bytes(),
            crate::CHUNK_VOLUME / 8 + 2 * size_of::<Overlay>()
        );
        assert_eq!(size_of::<Overlay>(), 30);
    }

    #[test]
    fn setting_the_uniform_overlay_does_not_allocate() {
        let mut dry = ChunkOverlay::none();
        dry.set(1, 1, 1, Overlay::NONE);
        assert_eq!(dry.heap_bytes(), 0);
    }
}
