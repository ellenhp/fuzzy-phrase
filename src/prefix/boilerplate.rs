use fst::automaton::{AlwaysMatch, Automaton};
use fst::raw;
use fst::Error as FstError;
use fst::FakeArrSlice;
use fst::Streamer;
use fst::Ulen;
use prefix::raw::Fst;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
#[cfg(feature = "mmap")]
use std::path::Path;

// pretty much everything in this file is copied from either upstream fst::Set or upstream
// fst:Map, so it's quarantined in its own file to separate it from stuff we're actually building
// ourselves (mostly operations relevant to prefixes)

pub struct PrefixSet(raw::Fst);

impl PrefixSet {
    // these are lifted from upstream Set
    #[cfg(feature = "mmap")]
    pub unsafe fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, FstError> {
        let mut buf = vec![];
        File::open(path).unwrap().read_to_end(&mut buf).unwrap();
        Fst::new(buf).map(PrefixSet)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, FstError> {
        Fst::new(bytes).map(PrefixSet)
    }

    pub fn from_iter<T, I>(iter: I) -> Result<Self, FstError>
    where
        T: AsRef<[u8]>,
        I: IntoIterator<Item = T>,
    {
        let mut builder = PrefixSetBuilder::memory();
        builder.extend_iter(iter)?;
        PrefixSet::from_bytes(builder.into_inner()?)
    }

    pub fn stream(&self) -> Stream {
        Stream::new(self.0.stream())
    }

    pub fn len(&self) -> Ulen {
        self.0.len()
    }

    pub fn as_fst(&self) -> &raw::Fst {
        &self.0
    }
}

// // Also from Map
// impl fmt::Debug for PrefixSet {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "PrefixSet([")?;
//         let mut stream = self.stream();
//         let mut first = true;
//         while let Some((k, v)) = stream.next() {
//             if !first {
//                 write!(f, ", ")?;
//             }
//             first = false;
//             let mut buf = vec![];
//             k.read(&mut buf).unwrap();
//             write!(f, "({}, {})", String::from_utf8_lossy(&buf), v)?;
//         }
//         write!(f, "])")
//     }
// }

pub struct PrefixSetBuilder<W> {
    builder: raw::Builder<W>,
    count: u64,
}

impl PrefixSetBuilder<Vec<u8>> {
    pub fn memory() -> Self {
        PrefixSetBuilder {
            builder: raw::Builder::memory(),
            count: 0,
        }
    }
}

impl<W: Write> PrefixSetBuilder<W> {
    pub fn new(wtr: W) -> Result<PrefixSetBuilder<W>, FstError> {
        Ok(PrefixSetBuilder {
            builder: raw::Builder::new_type(wtr, 0)?,
            count: 0,
        })
    }

    pub fn insert<K: AsRef<[u8]>>(&mut self, key: K) -> Result<(), FstError> {
        // this is the main behavior change vs. upstream: enforce autoincrementing IDs
        self.builder.insert(key, self.count)?;
        self.count += 1;
        Ok(())
    }

    pub fn extend_iter<T, I>(&mut self, iter: I) -> Result<(), FstError>
    where
        T: AsRef<[u8]>,
        I: IntoIterator<Item = T>,
    {
        for key in iter {
            // likewise, enforce counts
            self.builder.insert(key, self.count)?;
            self.count += 1;
        }
        Ok(())
    }

    pub fn finish(self) -> Result<(), FstError> {
        self.builder.finish()
    }

    pub fn into_inner(self) -> Result<W, FstError> {
        self.builder.into_inner()
    }
}

pub struct Stream<'s, A = AlwaysMatch>(raw::Stream<'s, A>)
where
    A: Automaton;

impl<'s, A: Automaton> Stream<'s, A> {
    #[doc(hidden)]
    pub fn new(fst_stream: raw::Stream<'s, A>) -> Self {
        Stream(fst_stream)
    }

    pub fn into_str_vec(self) -> Result<Vec<(String, u64)>, FstError> {
        self.0.into_str_vec()
    }
}

impl<'a, 's, A: Automaton> Streamer<'a> for Stream<'s, A> {
    type Item = (FakeArrSlice<'a>, u64);

    fn next(&'a mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, out)| (key, out.value()))
    }
}
