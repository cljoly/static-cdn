//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

use twox_hash::XxHash3_128;
use walkdir::WalkDir;

const SEED: u64 = 0x431C_71C5_AD99_39B4;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let _ = args.next().unwrap(); // Throw away the binary’s name
    for entry in WalkDir::new(args.next().unwrap()).max_open(dbg!(args.next().unwrap()).parse()?) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let path = entry.path();
            let mut f = File::open(path)?;
            let mut b = Vec::new();
            f.read_to_end(&mut b)?;
            let hash = XxHash3_128::oneshot_with_seed(SEED, &b);
            println!("{hash} {}", path.display());
        }
    }

    Ok(())
}
