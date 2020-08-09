use std::env;
use std::process::Command;
use std::path::Path;
use std::time::SystemTime;
use std::fs;
use std::cmp::{Ordering, PartialOrd};

use pairing_heap::PairingHeap;

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

enum SearchType {
    Directory,
    File,
    Editable,
    Other,
}

impl PartialEq for FileInfo {
    fn eq(&self, other: &Self) -> bool {
        self.filename == other.filename
    }
}

impl Eq for FileInfo {
}

impl PartialOrd for FileInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>{
        self.filename.partial_cmp(&other.filename)
    }
}

impl Ord for FileInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn main() {
    let home = env::var("HOME").unwrap();
    let fasd_file = format!("{}/.fasd", home);

    let mut args = env::args();
    let called_by_name = args.next().unwrap();
    let prog = args.next().unwrap();
    let mut substrings : Vec<String> = args.collect();
    //first argument may be a directory... if it's of type parent directory, transform it
    //appropriately so that rust can understand it. (Unlike bash it doesn't understand
    // `/home/guru/..`) so -> `/home`.  So we can have bash alias `e..=e ~+/..`
    //
    // With this use of `canonicalize`, we can have `e ..` (edit from parent directory) without any
    // special alias.
    // Additionally we have:
    // `e .` which acts as `e ~+` (previously we only had `e ~+`)
    // `e ~` edit from home directory.
    // `e ~-` edit from previous directory.
    let p = fs::canonicalize(&substrings[0]);
    if let Ok(path) = p {
        //if the directory contains file `abc` and we use `e abc` we may not necessarily mean that file. As we would have used `e . abc` or `e ~+ abc`
        //so better use it as path only if it's a directory.
        if path.is_dir() {
            substrings[0] = String::from(path.to_string_lossy());
        }
    }

    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32;
    let frecency_and_file_info = |finfo: FileInfo| {
        let frec = finfo.frecency(now) * 10000_f32;
        (frec as u32, finfo)
    };


    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .from_path(fasd_file).unwrap();

    let contains_all_substrings = |filename: &str| substrings
        .iter()
        .all(|part| filename.contains(part));

    let searched_for = if called_by_name.contains("dir") {
        SearchType::Directory
    } else if called_by_name.contains("file") {
        SearchType::File
    } else if called_by_name.contains("editable") {
        SearchType::Editable
    } else {
        SearchType::Other
    };

    let pre_filter: fn(&str) -> bool = match searched_for{
        SearchType::Directory => |_| true,
        SearchType::File => file_filter,
        SearchType::Editable => editable_filter,
        _ => |_| true,
    };
    let post_filter: fn(&str) -> bool = match searched_for{
        SearchType::Directory => dir_filter,
        SearchType::File => |_| true,
        SearchType::Editable => |_| true,
        _ => |filename|  Path::new(filename).exists()
    };

    let mut files = PairingHeap::new();
    for res in rdr.deserialize() {
        let finfo: FileInfo = res.unwrap();
        if contains_all_substrings(&finfo.filename) && pre_filter(&finfo.filename) {
                files.push(frecency_and_file_info(finfo));
        }
    }

    while let Some((_, FileInfo{filename, ..})) = files.peek() {
        if post_filter(filename) {
            Command::new(&prog).arg(filename).spawn().expect("Failed to execute program with relevant file");
            break;
        }
        files.pop();
    }
}

fn dir_filter(filename: &str) -> bool {
    match fs::metadata(filename){
        Ok(dir) => dir.is_dir(),
        _ => false
    }
}

fn file_filter(filename: &str) -> bool {
    match fs::metadata(filename){
        Ok(file) => file.is_file(),
        _ => false
    }
}

fn editable_filter(filename: &str) -> bool {
    if let Ok(md) = fs::metadata(filename){
        const EDITABLE_FILE_MAX_SIZE_CRITERION : u64= 10 * 1000; //10 kilos
        //Some source files like Python may be long: longer than EDITABLE_FILE_MAX_SIZE_CRITERION
        md.is_file() && !md.permissions().readonly() && (md.len() <  EDITABLE_FILE_MAX_SIZE_CRITERION || exceptionally_long(filename))
            //NOTE: this criterion is ok for now. later we must find some features like "bigram patterns" of first 100 characters... that is not as time-consuming as a `$(file thisfile)` invocation
    } else {
        false
    }
}

fn exceptionally_long(filename: &str) -> bool {
    filename.ends_with(".py") || filename.ends_with("rc") || filename.ends_with("installed")
}
