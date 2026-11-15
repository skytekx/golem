// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * A reader of zip archives, for the two jars of the reference of the API: the names of the files
 * of a jar, and the bytes of one file, stored or deflated, checked by the CRC-32 that the archive
 * names for it. It is written on the standard library alone, as CONTRIBUTING.md section 8 asks
 * before a crate from outside. The reader holds no pin of an archive: tests/fetch.py and the
 * notebook hold the hashes of the two jars, and whoever runs the tool by hand reads an archive
 * on their own word. So it takes no memory on the word of an archive beyond what the bytes of
 * the archive can fill.
 */

//! A reader of zip archives on the standard library.

use std::{
    collections::BTreeMap,
    fs,
    path::Path, //
};

/// The signature of the record that ends an archive, and its length without a comment.
const END_RECORD: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
const END_LENGTH: usize = 22;
/// The longest comment that may follow the end record.
const COMMENT_LIMIT: usize = 65_535;
/// The signature and the fixed length of an entry of the central directory.
const DIRECTORY_ENTRY: u32 = 0x0201_4b50;
const DIRECTORY_LENGTH: usize = 46;
/// The signature and the fixed length of the header before the bytes of a file.
const FILE_HEADER: u32 = 0x0403_4b50;
const HEADER_LENGTH: usize = 30;

/// A zip archive held in memory, with the files that its central directory names.
pub struct Archive {
    bytes: Vec<u8>,
    entries: BTreeMap<String, Entry>,
}

/// One file of an archive, as the central directory describes it.
struct Entry {
    method: u16,
    crc: u32,
    packed: usize,
    size: usize,
    header: usize,
}

impl Archive {
    /// Reads the archive at `path` and its central directory.
    pub fn open(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let entries = directory(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(Self { bytes, entries })
    }

    /// The bytes of the archive itself, for its hash.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The names of the files and directories of the archive, in the order of their bytes.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// The bytes of the file `name`, or `None` where the archive holds no such file.
    pub fn read(&self, name: &str) -> Result<Option<Vec<u8>>, String> {
        let Some(entry) = self.entries.get(name) else {
            return Ok(None);
        };
        if long_at(&self.bytes, entry.header)? != u64::from(FILE_HEADER) {
            return Err(format!(
                "{name}: no file header where the directory names one"
            ));
        }
        let start = entry.header
            + HEADER_LENGTH
            + usize::from(short_at(&self.bytes, entry.header + 26)?)
            + usize::from(short_at(&self.bytes, entry.header + 28)?);
        let packed = self
            .bytes
            .get(start..start + entry.packed)
            .ok_or_else(|| format!("{name}: its bytes end after the archive"))?;
        let data = match entry.method {
            0 => packed.to_vec(),
            8 => inflate(packed, entry.size).map_err(|error| format!("{name}: {error}"))?,
            other => return Err(format!("{name}: the method {other} is not read")),
        };
        if data.len() != entry.size || crc32(&data) != entry.crc {
            return Err(format!(
                "{name}: its bytes miss the size or the CRC-32 of the directory"
            ));
        }
        Ok(Some(data))
    }
}

/// The two bytes at `at` as a number, low byte first.
fn short_at(bytes: &[u8], at: usize) -> Result<u16, String> {
    match bytes.get(at..at + 2) {
        Some(&[low, high]) => Ok(u16::from_le_bytes([low, high])),
        _ => Err(String::from("the archive ends inside a record")),
    }
}

/// The four bytes at `at` as a number, low byte first.
fn long_at(bytes: &[u8], at: usize) -> Result<u64, String> {
    match bytes.get(at..at + 4) {
        Some(&[first, second, third, fourth]) => Ok(u64::from(u32::from_le_bytes([
            first, second, third, fourth,
        ]))),
        _ => Err(String::from("the archive ends inside a record")),
    }
}

/// The four bytes at `at` as a place or a length in the archive.
fn place_at(bytes: &[u8], at: usize) -> Result<usize, String> {
    let value = long_at(bytes, at)?;
    if value == u64::from(u32::MAX) {
        return Err(String::from("a zip64 archive is not read"));
    }
    usize::try_from(value).map_err(|_| String::from("a place beyond the memory of this machine"))
}

/// Reads the central directory of the archive in `bytes`.
fn directory(bytes: &[u8]) -> Result<BTreeMap<String, Entry>, String> {
    let last = bytes.len().saturating_sub(END_LENGTH);
    let first = last.saturating_sub(COMMENT_LIMIT);
    let end = (first..=last)
        .rev()
        .find(|&at| bytes[at..].starts_with(&END_RECORD))
        .ok_or("no end record of a zip archive")?;
    let count = short_at(bytes, end + 10)?;
    if count == u16::MAX {
        return Err(String::from("a zip64 archive is not read"));
    }
    let mut at = place_at(bytes, end + 16)?;
    let mut entries = BTreeMap::new();
    for _ in 0..count {
        if long_at(bytes, at)? != u64::from(DIRECTORY_ENTRY) {
            return Err(String::from("a central directory that ends early"));
        }
        let name_length = usize::from(short_at(bytes, at + 28)?);
        let name = bytes
            .get(at + DIRECTORY_LENGTH..at + DIRECTORY_LENGTH + name_length)
            .ok_or("the archive ends inside a name")?;
        let name = String::from_utf8(name.to_vec()).map_err(|_| "a name that is no UTF-8")?;
        let entry = Entry {
            method: short_at(bytes, at + 10)?,
            crc: u32::try_from(long_at(bytes, at + 16)?)
                .map_err(|_| "a CRC-32 of more than 32 bits")?,
            packed: place_at(bytes, at + 20)?,
            size: place_at(bytes, at + 24)?,
            header: place_at(bytes, at + 42)?,
        };
        if entries.insert(name.clone(), entry).is_some() {
            return Err(format!("the archive names {name} twice"));
        }
        at += DIRECTORY_LENGTH
            + name_length
            + usize::from(short_at(bytes, at + 30)?)
            + usize::from(short_at(bytes, at + 32)?);
    }
    Ok(entries)
}

/// The CRC-32 of zip, the one of IEEE 802.3, over `bytes`.
fn crc32(bytes: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (value, slot) in (0u32..).zip(table.iter_mut()) {
        let mut entry = value;
        for _ in 0..8 {
            entry = if entry & 1 == 1 {
                0xedb8_8320 ^ (entry >> 1)
            } else {
                entry >> 1
            };
        }
        *slot = entry;
    }
    let mut crc = u32::MAX;
    for byte in bytes {
        crc = table[usize::from(crc.to_le_bytes()[0] ^ byte)] ^ (crc >> 8);
    }
    !crc
}

/// The bits of a deflate stream, taken from the low bit of each byte up.
struct Bits<'a> {
    data: &'a [u8],
    at: usize,
    hold: u32,
    held: u32,
}

impl Bits<'_> {
    /// The next `count` bits as a number, at most 16 of them.
    fn take(&mut self, count: u32) -> Result<usize, String> {
        while self.held < count {
            let byte = self
                .data
                .get(self.at)
                .ok_or("a deflate stream that ends early")?;
            self.hold |= u32::from(*byte) << self.held;
            self.held += 8;
            self.at += 1;
        }
        let value = self.hold & ((1 << count) - 1);
        self.hold >>= count;
        self.held -= count;
        usize::try_from(value).map_err(|_| String::from("a value beyond this machine"))
    }
}

/// A canonical Huffman code: how many codes each length has, and the symbols in the order of
/// their codes.
struct Code {
    counts: [usize; 16],
    symbols: Vec<usize>,
}

impl Code {
    /// The code whose symbol `n` has the length `lengths[n]` in bits, 0 for a symbol without a
    /// code.
    fn new(lengths: &[usize]) -> Result<Self, String> {
        let mut counts = [0usize; 16];
        for &length in lengths {
            *counts.get_mut(length).ok_or("a code longer than 15 bits")? += 1;
        }
        // A code is refused where its lengths ask for more codes than the bits can tell apart.
        let mut free = 1usize;
        for count in &counts[1..] {
            free = (free * 2)
                .checked_sub(*count)
                .ok_or("a code that names too many symbols")?;
        }
        let mut next = [0usize; 16];
        for length in 1..15 {
            next[length + 1] = next[length] + counts[length];
        }
        let mut symbols = vec![0usize; lengths.len()];
        for (symbol, &length) in lengths.iter().enumerate() {
            if length != 0 {
                symbols[next[length]] = symbol;
                next[length] += 1;
            }
        }
        Ok(Self { counts, symbols })
    }

    /// Reads one symbol, a bit at a time, the shortest codes first.
    fn decode(&self, bits: &mut Bits<'_>) -> Result<usize, String> {
        let mut code = 0usize;
        let mut first = 0usize;
        let mut index = 0usize;
        for count in &self.counts[1..] {
            code |= bits.take(1)?;
            if code < first + count {
                return Ok(self.symbols[index + (code - first)]);
            }
            index += count;
            first = (first + count) * 2;
            code *= 2;
        }
        Err(String::from("a code that no symbol has"))
    }
}

/// The base and the extra bits of the 29 length symbols, and of the 30 distance symbols.
const LENGTH_BASE: [usize; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DISTANCE_BASE: [usize; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12_289, 16_385, 24_577,
];
const DISTANCE_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
/// The order in which a dynamic block gives the lengths of its code of code lengths.
const LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// The most bytes that one byte of a deflate stream can stand for: a match copies 258 bytes at
/// the most and takes two bits at the least, one for its length and one for its distance.
const GROWTH: usize = 1032;

/// Inflates the deflate stream `data` of RFC 1951, which holds `size` bytes.
fn inflate(data: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let mut bits = Bits {
        data,
        at: 0,
        hold: 0,
        held: 0,
    };
    // The directory names the size, and a directory may name any size. Deflate grows a byte
    // to GROWTH bytes at the most, so the packed bytes bound what is taken before any is read.
    let mut out = Vec::with_capacity(size.min(data.len().saturating_mul(GROWTH)));
    loop {
        let last = bits.take(1)? == 1;
        match bits.take(2)? {
            0 => stored(&mut bits, &mut out)?,
            1 => {
                let mut lengths = [8usize; 288];
                lengths[144..256].fill(9);
                lengths[256..280].fill(7);
                block(
                    &mut bits,
                    &mut out,
                    size,
                    &Code::new(&lengths)?,
                    &Code::new(&[5; 30])?,
                )?;
            }
            2 => {
                let (lengths, distances) = dynamic(&mut bits)?;
                block(&mut bits, &mut out, size, &lengths, &distances)?;
            }
            _ => return Err(String::from("a deflate block of an unknown kind")),
        }
        if out.len() > size {
            return Err(String::from("a deflate stream longer than its size"));
        }
        if last {
            return Ok(out);
        }
    }
}

/// Copies a stored block, which starts at the next whole byte with its length and the
/// complement of that length.
fn stored(bits: &mut Bits<'_>, out: &mut Vec<u8>) -> Result<(), String> {
    bits.hold = 0;
    bits.held = 0;
    let length = short_at(bits.data, bits.at)?;
    if short_at(bits.data, bits.at + 2)? != !length {
        return Err(String::from(
            "a stored block whose length misses its complement",
        ));
    }
    let start = bits.at + 4;
    let bytes = bits
        .data
        .get(start..start + usize::from(length))
        .ok_or("a stored block that ends early")?;
    out.extend_from_slice(bytes);
    bits.at = start + usize::from(length);
    Ok(())
}

/// Reads the two codes that a dynamic block opens with.
fn dynamic(bits: &mut Bits<'_>) -> Result<(Code, Code), String> {
    let literals = bits.take(5)? + 257;
    let distances = bits.take(5)? + 1;
    let given = bits.take(4)? + 4;
    if literals > 286 || distances > 30 {
        return Err(String::from("a dynamic block with too many codes"));
    }
    let mut order = [0usize; 19];
    for place in &LENGTH_ORDER[..given] {
        order[*place] = bits.take(3)?;
    }
    let order = Code::new(&order)?;
    let mut lengths = Vec::with_capacity(literals + distances);
    while lengths.len() < literals + distances {
        let (length, times) = match order.decode(bits)? {
            16 => {
                let previous = *lengths.last().ok_or("a repeat with no length before it")?;
                (previous, 3 + bits.take(2)?)
            }
            17 => (0, 3 + bits.take(3)?),
            18 => (0, 11 + bits.take(7)?),
            length => (length, 1),
        };
        lengths.extend(std::iter::repeat_n(length, times));
    }
    if lengths.len() > literals + distances {
        return Err(String::from(
            "a repeat that runs over the lengths of a block",
        ));
    }
    if lengths[256] == 0 {
        return Err(String::from("a dynamic block with no code for its end"));
    }
    Ok((
        Code::new(&lengths[..literals])?,
        Code::new(&lengths[literals..])?,
    ))
}

/// Inflates one block with the codes of its literals and lengths and of its distances.
fn block(
    bits: &mut Bits<'_>,
    out: &mut Vec<u8>,
    size: usize,
    lengths: &Code,
    distances: &Code,
) -> Result<(), String> {
    loop {
        let symbol = lengths.decode(bits)?;
        if let Ok(literal) = u8::try_from(symbol) {
            out.push(literal);
        } else if symbol == 256 {
            return Ok(());
        } else {
            let index = symbol - 257;
            let base = LENGTH_BASE.get(index).ok_or("a length symbol beyond 285")?;
            let length = base + bits.take(LENGTH_EXTRA[index])?;
            let index = distances.decode(bits)?;
            let base = DISTANCE_BASE
                .get(index)
                .ok_or("a distance symbol beyond 29")?;
            let distance = base + bits.take(DISTANCE_EXTRA[index])?;
            let start = out
                .len()
                .checked_sub(distance)
                .ok_or("a distance that reaches before the output")?;
            for place in start..start + length {
                out.push(out[place]);
            }
        }
        if out.len() > size {
            return Err(String::from("a deflate stream longer than its size"));
        }
    }
}
