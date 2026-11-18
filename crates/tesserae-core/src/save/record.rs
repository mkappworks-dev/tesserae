//! A pack's records: a kind and a version, then a record-local name table and the content, so a
//! record is self-contained and compaction copies its bytes. Integers are LEB128, signed ones
//! zigzagged first.

use std::collections::HashMap;
use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;

use super::{
    Cell, Error, FireChange, FireClock, MAX_PALETTE, MAX_RECORD_BYTES, Rescan, SavedChunk,
    SavedFluid, Solid, TimerState,
};
use crate::CHUNK_VOLUME;

pub(super) const CHUNK: u8 = 1;
pub(super) const SHARE: u8 = 2;
const VERSION: u8 = 1;

/// What a paused share holds of its own. The rest is the active simulation's, which resuming reads
/// nothing of but the fairness turn, and only when the active one has none.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PausedWork {
    pub timers: TimerState,
    pub wakes: Vec<([i32; 3], u64)>,
    pub rescans: Vec<Rescan>,
    pub landing_cursors: Vec<([i32; 3], [i32; 3])>,
}

fn corrupt(why: &str) -> Error {
    Error::Corrupt(format!("save record {why}"))
}

fn deflate(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let mut deflater = DeflateEncoder::new(Vec::new(), Compression::default());
    deflater.write_all(bytes)?;
    Ok(deflater.finish()?)
}

/// One byte past `limit` is read, so an overlong stream is refused without being inflated whole.
fn inflate(bytes: &[u8], limit: u64) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    DeflateDecoder::new(bytes)
        .take(limit + 1)
        .read_to_end(&mut out)
        .map_err(|_| corrupt("does not inflate"))?;
    if out.len() as u64 > limit {
        return Err(corrupt("is longer than it says"));
    }
    Ok(out)
}

/// Byte planes a palette's indices take: none for a palette of one, the high plane only past 256.
/// Measured on a flown World's Chunks, deflated planes came out 8% smaller than indices packed at
/// the fewest bits, which split a Chunk's long runs across byte boundaries.
fn planes(palette: usize) -> usize {
    match palette {
        0 | 1 => 0,
        2..=256 => 1,
        _ => 2,
    }
}

/// The low byte of every Voxel's index, then the high byte when the palette needs it, deflated.
pub(super) fn pack_indices(indices: &[u16], palette: usize) -> Result<Vec<u8>, Error> {
    let mut raw = Vec::with_capacity(indices.len() * planes(palette));
    for plane in 0..planes(palette) {
        raw.extend(indices.iter().map(|i| i.to_le_bytes()[plane]));
    }
    deflate(&raw)
}

pub(super) fn unpack_indices(packed: &[u8], palette: usize) -> Result<Vec<u16>, Error> {
    let planes = planes(palette);
    let raw = inflate(packed, (CHUNK_VOLUME * planes) as u64)?;
    if raw.len() != CHUNK_VOLUME * planes {
        return Err(corrupt("does not fill one Chunk"));
    }
    Ok((0..CHUNK_VOLUME)
        .map(|i| {
            let byte = |plane: usize| {
                if plane < planes {
                    raw[plane * CHUNK_VOLUME + i]
                } else {
                    0
                }
            };
            u16::from_le_bytes([byte(0), byte(1)])
        })
        .collect())
}

/// Content being written, with the names it uses gathered into its table.
#[derive(Default)]
struct Out {
    bytes: Vec<u8>,
    names: Vec<String>,
    index: HashMap<String, u64>,
}

impl Out {
    fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }

    fn uint(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.bytes.push(v as u8 | 0x80);
            v >>= 7;
        }
        self.bytes.push(v as u8);
    }

    fn int(&mut self, v: i64) {
        self.uint(((v << 1) ^ (v >> 63)) as u64);
    }

    fn cell(&mut self, at: [i32; 3]) {
        for c in at {
            self.int(i64::from(c));
        }
    }

    fn span(&mut self, (min, max): ([i32; 3], [i32; 3])) {
        self.cell(min);
        self.cell(max);
    }

    fn name(&mut self, name: &str) {
        let next = self.names.len() as u64;
        let at = *self.index.entry(name.to_owned()).or_insert_with(|| {
            self.names.push(name.to_owned());
            next
        });
        self.uint(at);
    }

    fn solid(&mut self, s: &Solid) {
        self.name(&s.material);
        self.name(&s.shape);
        self.u8(s.orientation);
    }

    fn fluid(&mut self, f: &SavedFluid) {
        self.name(&f.material);
        self.u8(f.source.into());
        for level in f.levels.iter().flatten() {
            self.uint((*level).into());
        }
    }

    fn some<T>(&mut self, v: Option<&T>, write: impl FnOnce(&mut Self, &T)) {
        self.u8(v.is_some().into());
        if let Some(v) = v {
            write(self, v);
        }
    }

    fn list<T>(&mut self, items: &[T], mut write: impl FnMut(&mut Self, &T)) {
        self.uint(items.len() as u64);
        for item in items {
            write(self, item);
        }
    }

    /// The name table, then the content.
    fn finish(self) -> Vec<u8> {
        let mut head = Self::default();
        head.uint(self.names.len() as u64);
        for name in &self.names {
            head.uint(name.len() as u64);
            head.bytes.extend_from_slice(name.as_bytes());
        }
        head.bytes.extend(self.bytes);
        head.bytes
    }
}

/// Content being read. Every read is bounded by what is left, so a damaged record is refused as
/// corrupt rather than panicking or allocating what it claims.
struct In<'a> {
    bytes: &'a [u8],
    at: usize,
    names: Vec<String>,
}

impl<'a> In<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, Error> {
        let mut r = Self {
            bytes,
            at: 0,
            names: Vec::new(),
        };
        let count = r.count()?;
        for _ in 0..count {
            let len = r.count()?;
            let name = std::str::from_utf8(r.take(len)?).map_err(|_| corrupt("names no text"))?;
            r.names.push(name.to_owned());
        }
        Ok(r)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let end = self
            .at
            .checked_add(len)
            .filter(|&end| end <= self.bytes.len())
            .ok_or_else(|| corrupt("is truncated"))?;
        let taken = &self.bytes[self.at..end];
        self.at = end;
        Ok(taken)
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    fn bool(&mut self) -> Result<bool, Error> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(corrupt("holds an invalid flag")),
        }
    }

    fn uint(&mut self) -> Result<u64, Error> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = self.u8()?;
            v |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(corrupt("holds an overlong integer"))
    }

    fn int(&mut self) -> Result<i64, Error> {
        let v = self.uint()?;
        Ok((v >> 1) as i64 ^ -((v & 1) as i64))
    }

    fn i32(&mut self) -> Result<i32, Error> {
        i32::try_from(self.int()?).map_err(|_| corrupt("holds a coordinate out of range"))
    }

    fn u16(&mut self) -> Result<u16, Error> {
        u16::try_from(self.uint()?).map_err(|_| corrupt("holds a value out of range"))
    }

    fn usize(&mut self) -> Result<usize, Error> {
        usize::try_from(self.uint()?).map_err(|_| corrupt("holds a value out of range"))
    }

    /// A length, never more than the bytes left, since every item takes at least one.
    fn count(&mut self) -> Result<usize, Error> {
        let n = self.usize()?;
        if n > self.bytes.len() - self.at {
            return Err(corrupt("claims more than it holds"));
        }
        Ok(n)
    }

    fn cell(&mut self) -> Result<[i32; 3], Error> {
        Ok([self.i32()?, self.i32()?, self.i32()?])
    }

    fn span(&mut self) -> Result<([i32; 3], [i32; 3]), Error> {
        Ok((self.cell()?, self.cell()?))
    }

    fn name(&mut self) -> Result<String, Error> {
        let at = self.usize()?;
        self.names
            .get(at)
            .cloned()
            .ok_or_else(|| corrupt("names past its table"))
    }

    fn solid(&mut self) -> Result<Solid, Error> {
        Ok(Solid {
            material: self.name()?,
            shape: self.name()?,
            orientation: self.u8()?,
        })
    }

    fn fluid(&mut self) -> Result<SavedFluid, Error> {
        let material = self.name()?;
        let source = self.bool()?;
        let mut levels = [[0u16; 3]; 4];
        for level in levels.iter_mut().flatten() {
            *level = self.u16()?;
        }
        Ok(SavedFluid {
            material,
            source,
            levels,
        })
    }

    fn some<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<Option<T>, Error> {
        self.bool()?.then(|| read(self)).transpose()
    }

    fn list<T>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let n = self.count()?;
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            items.push(read(self)?);
        }
        Ok(items)
    }

    fn end(&self) -> Result<(), Error> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(corrupt("has bytes past its content"))
        }
    }
}

fn header(kind: u8) -> Vec<u8> {
    vec![kind, VERSION]
}

fn body(kind: u8, record: &[u8]) -> Result<&[u8], Error> {
    match record {
        [k, VERSION, rest @ ..] if *k == kind => Ok(rest),
        _ => Err(corrupt("is of another kind or version")),
    }
}

/// The kind, the version, the length of the deflated table, coordinate, revision and palette,
/// then them, then the deflated indices as [`SavedChunk::capture`] packed them, copied as they are.
pub(super) fn chunk(saved: &SavedChunk) -> Result<Vec<u8>, Error> {
    let mut out = Out::default();
    out.cell(saved.coord);
    out.uint(saved.revision);
    let (palette, packed) = saved.packed()?;
    out.list(&palette, |out, cell| {
        out.solid(&cell.solid);
        out.some(cell.fluid.as_ref(), Out::fluid);
        out.some(cell.fuel.as_ref(), |out, fuel| out.uint((*fuel).into()));
    });
    let head = deflate(&out.finish())?;
    let mut record = header(CHUNK);
    let mut length = Out::default();
    length.uint(head.len() as u64);
    record.extend(length.bytes);
    record.extend(head);
    record.extend_from_slice(&packed);
    Ok(record)
}

pub(super) fn read_chunk(record: &[u8]) -> Result<SavedChunk, Error> {
    let rest = body(CHUNK, record)?;
    let mut length = In {
        bytes: rest,
        at: 0,
        names: Vec::new(),
    };
    let head_len = length.count()?;
    let head = length.take(head_len)?;
    let packed = &rest[length.at..];
    let head = inflate(head, MAX_RECORD_BYTES)?;
    let mut r = In::new(&head)?;
    let coord = r.cell()?;
    let revision = r.uint()?;
    let palette = r.list(|r| {
        Ok(Cell {
            solid: r.solid()?,
            fluid: r.some(In::fluid)?,
            fuel: r.some(In::u16)?,
        })
    })?;
    r.end()?;
    if palette.is_empty() || palette.len() > MAX_PALETTE || packed.is_empty() {
        return Err(corrupt("holds no palette, too large a one or no indices"));
    }
    Ok(SavedChunk {
        coord,
        revision,
        runs: Vec::new(),
        palette,
        cells: String::new(),
        packed: packed.to_vec(),
    })
}

fn clock(out: &mut Out, clock: &FireClock) {
    match clock {
        FireClock::Running { remaining } => {
            out.u8(0);
            out.uint(*remaining);
        }
        FireClock::Frozen { remaining, solid } => {
            out.u8(1);
            out.uint(*remaining);
            out.solid(solid);
        }
    }
}

fn change(out: &mut Out, change: &FireChange) {
    match change {
        FireChange::Ignite { fuel } => {
            out.u8(0);
            out.uint((*fuel).into());
        }
        FireChange::Extinguish => out.u8(1),
        FireChange::Burnout { residue } => {
            out.u8(2);
            out.solid(residue);
        }
    }
}

/// The kind, the version, then the deflated table and timers, wakes, rescans and landing
/// cursors.
pub(super) fn share(work: &PausedWork) -> Result<Vec<u8>, Error> {
    let mut out = Out::default();
    out.list(&work.timers.flow, |out, (at, remaining, want)| {
        out.cell(*at);
        out.uint(*remaining);
        out.some(want.as_ref(), Out::fluid);
    });
    out.list(&work.timers.fire, |out, (at, c, ch)| {
        out.cell(*at);
        clock(out, c);
        change(out, ch);
    });
    out.list(&work.wakes, |out, (at, remaining)| {
        out.cell(*at);
        out.uint(*remaining);
    });
    out.list(&work.rescans, |out, r| {
        out.cell(r.home);
        out.list(&r.pass, |out, s| out.span(*s));
        out.uint(r.span as u64);
        out.uint(r.offset as u64);
        out.list(&r.next, |out, s| out.span(*s));
        out.some(r.parked.as_ref(), |out, s| out.span(*s));
    });
    out.list(&work.landing_cursors, |out, (home, at)| {
        out.cell(*home);
        out.cell(*at);
    });
    let mut record = header(SHARE);
    record.extend(deflate(&out.finish())?);
    Ok(record)
}

pub(super) fn read_share(record: &[u8]) -> Result<PausedWork, Error> {
    let content = inflate(body(SHARE, record)?, MAX_RECORD_BYTES)?;
    let mut r = In::new(&content)?;
    let flow = r.list(|r| Ok((r.cell()?, r.uint()?, r.some(In::fluid)?)))?;
    let fire = r.list(|r| {
        let at = r.cell()?;
        let clock = match r.u8()? {
            0 => FireClock::Running {
                remaining: r.uint()?,
            },
            1 => FireClock::Frozen {
                remaining: r.uint()?,
                solid: r.solid()?,
            },
            _ => return Err(corrupt("holds an unknown fire clock")),
        };
        let change = match r.u8()? {
            0 => FireChange::Ignite { fuel: r.u16()? },
            1 => FireChange::Extinguish,
            2 => FireChange::Burnout {
                residue: r.solid()?,
            },
            _ => return Err(corrupt("holds an unknown fire change")),
        };
        Ok((at, clock, change))
    })?;
    let wakes = r.list(|r| Ok((r.cell()?, r.uint()?)))?;
    let rescans = r.list(|r| {
        Ok(Rescan {
            home: r.cell()?,
            pass: r.list(In::span)?,
            span: r.usize()?,
            offset: r.usize()?,
            next: r.list(In::span)?,
            parked: r.some(In::span)?,
        })
    })?;
    let landing_cursors = r.list(|r| Ok((r.cell()?, r.cell()?)))?;
    r.end()?;
    Ok(PausedWork {
        timers: TimerState { flow, fire },
        wakes,
        rescans,
        landing_cursors,
    })
}
