//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::error::Error;
use std::fs::File;
use std::hash::Hasher as _;
use std::io::Read;

use rayon::prelude::*;

use twox_hash::XxHash64;
use walkdir::WalkDir;

const SEED: u64 = 0x431C_71C5_AD99_39B4;
const CHUNK_SIZE: usize = 1 << 16;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let _ = args.next().unwrap(); // Throw away the binary’s name
    let root_dir = args.next().unwrap();
    let all_files = WalkDir::new(root_dir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                Some(entry)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    all_files.par_iter().for_each(|entry| {
        let path = entry.path();
        let mut f = File::open(path).unwrap();
        let mut b = [0u8; CHUNK_SIZE];
        let mut hasher = XxHash64::with_seed(SEED);
        loop {
            let n = f.read(&mut b).unwrap();
            // This will hash trailing null bytes, but it's fine: if a file differs only by null
            // bytes, for our purpose, we can deem it equal.
            hasher.write(&b);
            if n == 0 {
                break;
            }
        }
        let hash = hasher.finish();

        let metadata = path.metadata().unwrap();
        println!("{hash} {:?}:{:?}:{}", metadata.modified().unwrap(), metadata.len(), path.display());
    });

    Ok(())
}
