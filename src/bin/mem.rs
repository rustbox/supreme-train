use std::{
    fmt::Debug,
    ops::{Deref, RangeInclusive},
};

use object::{Object, ObjectSection, ObjectSymbol, SymbolIterator};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("requires argument (path to elf)");

    let data = std::fs::read(path).expect("read");

    let object = object::File::parse(data.as_slice()).expect("parse");

    print_alloc(&object, DRAM, "DRAM");
    print_alloc(&object, IRAM, "IRAM");
}

fn print_alloc<'data: 'file, 'file, 'str, Object>(object: &'file Object, r: Region, name: &'str str)
where
    Object: object::Object<'data, 'file> + 'file,
    Object::Symbol: Debug,
{
    let mut syms = object
        .symbols()
        .filter(|sym| r.contains(&sym.address()))
        .filter_map(|sym| sym.name().map(|name| (sym.address(), name, sym)).ok())
        .collect::<Vec<_>>();

    syms.sort_by(|&(aa, an, _), &(ba, bn, _)| aa.cmp(&ba).then(an.cmp(bn).reverse()));

    // for (addr, name, sym) in syms {
    //     println!("{addr:x} {name} {sym:?}")
    // }

    let rsz = r.end - r.start;
    let mut total = Bytes(0u64);
    let mut last = r.start;
    println!("{name} allocation:");
    println!(
        "{:21}\t{:7}\t({:^13})\t{:>8}\t{:<}",
        "addr", "size", "bytes", "%", "name"
    );
    for sec in object.sections().filter(|s| r.contains(&s.address())) {
        let align = sec.align();
        // align 1 -> x...xxx y...yyy -> 00
        // align 2 -> x...xx0 y...yyy -> 2 - 0y
        // align 4 -> x...x00 y...yyy -> 4 - yy

        let mask = align - 1;
        // let mask = (mask - 1) & !mask; // 0000 or 0011

        let pad = last & mask;
        if pad > 0 {
            println!(
                "{:>21}\t0x{align:05x}\t{:>11}\t{:>8}\t{:<}",
                "(padding)", "—", "—", "—"
            );
        }
        let name = sec.name().expect("non utf-8 section name");

        let start = sec.address();
        let sz = Bytes(sec.size());
        let end = start + sz;
        let alloc = sz.0 as f64 / rsz as f64 * 100f64;

        println!("0x{start:x}-0x{end:x}\t0x{sz:05x}\t({sz:6})\t{alloc:7.3}%\t{name}");
        total += sz + pad;
        last = end;
    }

    let alloc = total.0 as f64 / rsz as f64 * 100f64;
    println!(
        "{:>21}\t0x{total:05x}\t({total:6})\t{alloc:7.3}%\t—",
        "total"
    )
}

#[derive(Clone, Copy, Debug, Default)]
struct Bytes(u64);

impl std::fmt::Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // https://stackoverflow.com/questions/72544241/how-to-implement-display-while-respecting-width-fill-and-alignment
        let Self(d) = self;

        let (d, rem, suf) = [
            (60, "EiB"),
            (50, "PiB"),
            (40, "TiB"),
            (30, "GiB"),
            (20, "MiB"),
            (10, "KiB"),
        ]
        .into_iter()
        .find_map(|(shift, suf)| {
            if (d >> shift) > 0 {
                let div = 1 << shift;
                Some((d >> shift, (d % div) as f64 / div as f64, suf))
            } else {
                None
            }
        })
        .unwrap_or((*d, 0f64, "B"));
        // TODO it sure would be swell to be able to subtract some amount of width before formatting `d` here, while respecting all the other flags
        std::fmt::Display::fmt(&d, f).and_then(|_| write!(f, ".{:03.0}{:3}", rem * 1000.0, suf))
    }
}

impl std::fmt::LowerHex for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(d) = self;
        <u64 as std::fmt::LowerHex>::fmt(d, f)
    }
}

impl std::ops::Add<Bytes> for u64 {
    type Output = u64;

    fn add(self, rhs: Bytes) -> Self::Output {
        let Bytes(d) = rhs;
        self + d
    }
}

impl std::ops::Add<u64> for Bytes {
    type Output = Bytes;

    fn add(self, rhs: u64) -> Self::Output {
        let Bytes(d) = self;
        Bytes(d + rhs)
    }
}

impl std::ops::AddAssign<Bytes> for Bytes {
    fn add_assign(&mut self, rhs: Bytes) {
        let (Self(d), Self(rhs)) = (self, rhs);
        *d += rhs;
    }
}

type Region = std::ops::Range<u64>;

// cf. https://github.com/rustbox/esp-hal/blob/8815e752506903bcc37fe884ad9d9c8fe00ae75d/esp32c3-hal/ld/db-esp32c3-memory.x
const DRAM: Region = 0x3FC80000..(0x3FC80000 + 0x50000 + 0x600);
const IRAM: Region = (0x4037C000 + 0x4000)..((0x4037C000 + 0x4000) + (400 * 1024 - 0x400));
// const DROM: std::ops::Range<u64> = 0x3C000000..(0x3C000000 + 0x400000);

// these are the symbols that get emitted at the various locations
// const DRAM_REGIONS: &[[&str; 3]] = &[
//     [".data", "_sdata", "_edata"],
//     // .rwtext.dummy is where these actually get loaded
//     // [".rwtext", "", ""],
//     [".bss", "_sbss", "_ebss"],
//     [".uninit", "_suninit", "_euninit"],
//     [".heap", "_sheap", "_eheap"],
//     // note that stacks grow towards lower addresses
//     [".stack", "_estack", "_sstack"],
// ];

// struct Region {

// }

// struct MemMap {
//     DRAM: Region,
// }

// const MEMORY: MemMap = MemMap {};
