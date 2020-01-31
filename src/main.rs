use std::env;
use std::process::Command;
use std::path::Path;
use std::time::SystemTime;
use std::fs;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FileInfo{
    filename: String,
    rank: f32,
    timestamp: u32,
}

impl FileInfo {
    pub fn frecency(&self, now : u32) -> f32 {
        let duration = now - self.timestamp;
        let coef = match duration{
            d if d < 3600 => 6,
            d if d < 3600 * 24 => 4,
            d if d < 3600 * 24 * 7 => 2,
            _ => 1,
        };
        self.rank * coef as f32
    }
}

fn main() {
    let home = env::var("HOME").unwrap();
    let fasd_file = format!("{}/.fasd", home);

    let mut args = env::args();
    let called_by_name = args.next().unwrap();
    let prog = args.next().unwrap();
    let substrings : Vec<String> = args.collect();

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .from_path(fasd_file).unwrap();

    let contains_all_substrings = |filename: &str| substrings
        .iter()
        .all(|part| filename.contains(part));

    let mut files = Vec::new();
    for res in rdr.deserialize() {
        let finfo: FileInfo = res.unwrap();
        if contains_all_substrings(finfo.filename.as_ref()) && Path::new(&finfo.filename).exists() {
            if called_by_name.contains("dir"){
                if fs::metadata(&finfo.filename).unwrap().is_dir() {
                    files.push(finfo);
                }
            } else if called_by_name.contains("file"){ //NOTE: Soon we must have something called editable, that avoids binaries, pdfs etc...
                if fs::metadata(&finfo.filename).unwrap().is_file() {
                    files.push(finfo);
                }
            } else {
                    files.push(finfo);
            }
        }
    }

    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32;
    let orf = files
        .iter()
        .max_by_key(|f| (f.frecency(now) * 10000 as f32) as u32); // As there is no ordering in float...
    let orf = orf.unwrap();
    Command::new(&prog).arg(&orf.filename).spawn().expect("Failed to execute program with relevant file");
    // Command::new("fasd").arg("-A").arg(&orf.filename).spawn().expect("Failed to add to fasd");
}
