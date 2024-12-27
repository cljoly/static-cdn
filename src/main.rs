//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

use rayon::prelude::*;

use twox_hash::XxHash64;
use walkdir::WalkDir;

const SEED: u64 = 0x431C_71C5_AD99_39B4;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let _ = args.next().unwrap(); // Throw away the binary’s name
    let all_files = WalkDir::new(args.next().unwrap())
        .max_open(args.next().unwrap().parse()?)
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
        let mut b = Vec::new();
        f.read_to_end(&mut b).unwrap();
        let hash = XxHash64::oneshot(SEED, &b);
        println!("{hash} {}", path.display());
    });

    Ok(())
}
