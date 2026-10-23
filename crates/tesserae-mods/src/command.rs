//! What a Mod asks for, in metres, and the same asked-for in Voxels once the host has checked it.
//!
//! Spec decision 8. [`validate`] is pure and takes the tables and the World's Voxel size, so the
//! Server and the benches call exactly what a tick does.

use std::fmt;

use tesserae_core::{MaterialId, PlaceFault, Tables, Voxel};

use crate::adapt::Line;
use crate::limits::MAX_COMMANDS;

/// Voxels in one `fill`: a cost cap and not a size, so a region of the same metres is 64 times
/// as many at 0.25 m.
pub const MAX_FILL_VOXELS: u64 = 32_768;
/// Voxels in one batch, `fill`s and `place`s together.
pub const MAX_VOXELS_PER_CALL: u64 = 65_536;
/// Keeps `+ 1` and a neighbour's index inside `i32` at the far end of the lattice.
pub const MAX_VOXEL_INDEX: i32 = 1 << 30;

/// A region in metres, `[min, max)`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Region {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Placement {
    pub at: [f32; 3],
    pub voxel: Voxel,
}

/// One thing a Mod asked for, exactly as it asked.
#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    Fill {
        region: Region,
        voxel: Voxel,
    },
    Place(Vec<Placement>),
    PlaceSources {
        region: Region,
        material: MaterialId,
    },
    RemoveSources(Region),
    Ignite(Region),
    Extinguish(Region),
}

/// An inclusive box of Voxel indices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VoxelBox {
    pub min: [i32; 3],
    pub max: [i32; 3],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegionError {
    NotFinite,
    /// `min` is above `max` on some axis.
    Unordered,
    /// An index past [`MAX_VOXEL_INDEX`].
    OutOfRange,
    InvalidVoxelSize,
}

impl fmt::Display for RegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RegionError::NotFinite => "a coordinate is not finite",
            RegionError::Unordered => "min is above max",
            RegionError::OutOfRange => "a coordinate is too far from the origin",
            RegionError::InvalidVoxelSize => "Voxel size must be finite and positive",
        })
    }
}

/// The Voxel containing a point, by the floor `WorldSettings::voxel_at` takes.
pub fn voxel_index(at: [f32; 3], voxel_size: f32) -> Result<[i32; 3], RegionError> {
    check_voxel_size(voxel_size)?;
    let mut index = [0; 3];
    for (out, metres) in index.iter_mut().zip(at) {
        *out = bounded((metres / voxel_size).floor())?;
    }
    Ok(index)
}

fn check_voxel_size(size: f32) -> Result<(), RegionError> {
    if !size.is_finite() || size <= 0.0 || !size.recip().is_finite() {
        return Err(RegionError::InvalidVoxelSize);
    }
    Ok(())
}

fn bounded(index: f32) -> Result<i32, RegionError> {
    if !index.is_finite() {
        return Err(RegionError::NotFinite);
    }
    if index.abs() > MAX_VOXEL_INDEX as f32 {
        return Err(RegionError::OutOfRange);
    }
    Ok(index as i32)
}

impl VoxelBox {
    /// The Voxels whose lower corner is in `region`, which is the same physical set at any Voxel
    /// size. `None` when there are none, which a box that is inclusive cannot say.
    pub fn of(region: &Region, voxel_size: f32) -> Result<Option<VoxelBox>, RegionError> {
        check_voxel_size(voxel_size)?;
        let mut first = [0; 3];
        let mut last = [0; 3];
        for axis in 0..3 {
            let (min, max) = (region.min[axis], region.max[axis]);
            if !min.is_finite() || !max.is_finite() {
                return Err(RegionError::NotFinite);
            }
            if min > max {
                return Err(RegionError::Unordered);
            }
            first[axis] = bounded((min / voxel_size).ceil())?;
            last[axis] = bounded((max / voxel_size).ceil())? - 1;
        }
        if (0..3).any(|axis| last[axis] < first[axis]) {
            return Ok(None);
        }
        Ok(Some(VoxelBox {
            min: first,
            max: last,
        }))
    }

    pub fn count(&self) -> u64 {
        (0..3)
            .map(|axis| (i64::from(self.max[axis]) - i64::from(self.min[axis]) + 1) as u128)
            .product::<u128>()
            .try_into()
            .unwrap_or(u64::MAX)
    }

    /// A cursor over the box, x fastest. It is what a queue holds for a `fill`: the box and a
    /// position, not the Voxels.
    pub fn cursor(&self) -> BoxCursor {
        BoxCursor {
            region: *self,
            next: Some(self.min),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BoxCursor {
    region: VoxelBox,
    next: Option<[i32; 3]>,
}

impl BoxCursor {
    pub fn remaining(&self) -> u64 {
        let Some(at) = self.next else { return 0 };
        let (min, max) = (self.region.min, self.region.max);
        let row = i64::from(max[0]) - i64::from(min[0]) + 1;
        let layer = row * (i64::from(max[2]) - i64::from(min[2]) + 1);
        let done = i64::from(at[1] - min[1]) * layer
            + i64::from(at[2] - min[2]) * row
            + i64::from(at[0] - min[0]);
        self.region.count() - done as u64
    }
}

impl Iterator for BoxCursor {
    type Item = [i32; 3];

    fn next(&mut self) -> Option<[i32; 3]> {
        let at = self.next?;
        let (min, max) = (self.region.min, self.region.max);
        self.next = if at[0] < max[0] {
            Some([at[0] + 1, at[1], at[2]])
        } else if at[2] < max[2] {
            Some([min[0], at[1], at[2] + 1])
        } else if at[1] < max[1] {
            Some([min[0], at[1] + 1, min[2]])
        } else {
            None
        };
        Some(at)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = usize::try_from(self.remaining()).unwrap_or(usize::MAX);
        (remaining, Some(remaining))
    }
}

/// What a Command asks of one Voxel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Intent {
    /// A whole-Voxel placement through the solid-layer rule; air breaks. A 0.1
    /// placement of a fluid Material takes this path too, and becomes a source
    /// in place of whatever was there, as it did before fluids flowed.
    Voxel(Voxel),
    Source(MaterialId),
    RemoveSource,
    Ignite,
    Extinguish,
}

/// A [`Command`] the host has checked and put in Voxels.
#[derive(Clone, PartialEq, Debug)]
pub enum Resolved {
    Fill { region: VoxelBox, intent: Intent },
    Place(Vec<([i32; 3], Voxel)>),
}

/// A batch that passed [`validate`]: plain owned data, nothing left to check.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct CommandBatch {
    pub commands: Vec<Resolved>,
    /// The Voxels the batch writes, `fill`s and `place`s together.
    pub voxels: u64,
}

/// What the Mod is told of its previous batch.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct BatchReport {
    pub applied: u32,
    pub rejected: u32,
}

impl BatchReport {
    /// A refused batch counts as its Commands, since none of them was applied.
    pub fn refused(commands: usize) -> BatchReport {
        BatchReport {
            applied: 0,
            rejected: u32::try_from(commands).unwrap_or(u32::MAX),
        }
    }
}

/// Why a batch was refused whole. `command` is an index into the batch.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BatchError {
    TooManyCommands { count: usize },
    Region { command: usize, error: RegionError },
    FillTooLarge { command: usize, voxels: u64 },
    TooManyVoxels,
    Voxel { command: usize, fault: PlaceFault },
}

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchError::TooManyCommands { count } => {
                write!(f, "{count} commands, at most {MAX_COMMANDS} are allowed")
            }
            BatchError::Region { command, error } => write!(f, "command {command}: {error}"),
            BatchError::FillTooLarge { command, voxels } => write!(
                f,
                "command {command}: a region of {voxels} Voxels, at most {MAX_FILL_VOXELS} are allowed"
            ),
            BatchError::TooManyVoxels => {
                write!(f, "more than {MAX_VOXELS_PER_CALL} Voxels in one batch")
            }
            BatchError::Voxel { command, fault } => write!(f, "command {command}: {fault}"),
        }
    }
}

impl std::error::Error for BatchError {}

/// Checks a whole batch and returns it in Voxels, or refuses all of it. `voxel_size` is the
/// World's, in metres. A 0.2 solid placement of a fluid Material is refused: that line places
/// fluid as sources.
pub fn validate(
    commands: &[Command],
    line: Line,
    tables: &Tables,
    voxel_size: f32,
) -> Result<CommandBatch, BatchError> {
    if commands.len() > MAX_COMMANDS {
        return Err(BatchError::TooManyCommands {
            count: commands.len(),
        });
    }
    let solid = |voxel: Voxel, command| -> Result<(), BatchError> {
        vet(tables, voxel, command)?;
        if line != Line::V01 && tables.fluids.contains(voxel.material()) {
            return Err(BatchError::Voxel {
                command,
                fault: PlaceFault::NotSolid,
            });
        }
        Ok(())
    };
    let mut voxels = 0u64;
    let mut resolved = Vec::with_capacity(commands.len());
    for (command, item) in commands.iter().enumerate() {
        let (region, intent) = match item {
            Command::Place(placements) => {
                voxels += placements.len() as u64;
                if voxels > MAX_VOXELS_PER_CALL {
                    return Err(BatchError::TooManyVoxels);
                }
                let mut placed = Vec::with_capacity(placements.len());
                for placement in placements {
                    solid(placement.voxel, command)?;
                    let at = voxel_index(placement.at, voxel_size)
                        .map_err(|error| BatchError::Region { command, error })?;
                    placed.push((at, placement.voxel));
                }
                resolved.push(Resolved::Place(placed));
                continue;
            }
            Command::Fill { region, voxel } => {
                // Filling with air clears, and a Voxel that is air carries no ids to check.
                if !voxel.is_air() {
                    solid(*voxel, command)?;
                }
                (region, Intent::Voxel(*voxel))
            }
            Command::PlaceSources { region, material } => {
                if !tables.fluids.contains(*material) {
                    let fault = if tables.materials.contains(*material) {
                        PlaceFault::NotAFluid
                    } else {
                        PlaceFault::UnknownMaterial
                    };
                    return Err(BatchError::Voxel { command, fault });
                }
                (region, Intent::Source(*material))
            }
            Command::RemoveSources(region) => (region, Intent::RemoveSource),
            Command::Ignite(region) => (region, Intent::Ignite),
            Command::Extinguish(region) => (region, Intent::Extinguish),
        };
        let region = VoxelBox::of(region, voxel_size)
            .map_err(|error| BatchError::Region { command, error })?;
        let Some(region) = region else { continue };
        let count = region.count();
        if count > MAX_FILL_VOXELS {
            return Err(BatchError::FillTooLarge {
                command,
                voxels: count,
            });
        }
        voxels += count;
        if voxels > MAX_VOXELS_PER_CALL {
            return Err(BatchError::TooManyVoxels);
        }
        resolved.push(Resolved::Fill { region, intent });
    }
    Ok(CommandBatch {
        commands: resolved,
        voxels,
    })
}

fn vet(tables: &Tables, voxel: Voxel, command: usize) -> Result<(), BatchError> {
    tables
        .vet_placed(voxel)
        .map_err(|fault| BatchError::Voxel { command, fault })
}
