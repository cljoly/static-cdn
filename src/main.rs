use std::env;
use std::error::Error;

use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let _ = args.next().unwrap(); // Throw away the binary’s name
    for entry in WalkDir::new(args.next().unwrap()).max_open(dbg!(args.next().unwrap()).parse()?) {
        let entry = entry.unwrap();
        println!("{}", entry.path().display());
    }

    Ok(())
}
