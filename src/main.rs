use std::env;
use std::process::Command;
use std::path::Path;
use std::time::SystemTime;
use std::fs;
use regex::Regex;
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
        if self.filename < other.filename{
            Some(Ordering::Less)
        } else if self.filename > other.filename {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
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
    let parent_dir_regex = Regex::new("/[^/]+/[.][.]$").unwrap();
    //first argument may be a directory... if it's of type parent directory, transform it
    //appropriately so that rust can understand it. (Unlike bash it doesn't understand
    // `/home/guru/..`) so -> `/home`.  So we can have bash alias `e..=e ~+/..`
    substrings[0] = parent_dir_regex.replace(&substrings[0], "").to_string();

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

    let pre_filter: Option<fn(&str) -> bool> = match searched_for{
        SearchType::Directory => Some(dir_filter),
        SearchType::File => Some(file_filter),
        SearchType::Editable => Some(editable_filter),
        _ => None
    };
    let post_filter: Option<fn(&str) -> bool> = match searched_for{
        SearchType::Directory => None,
        SearchType::File => None,
        SearchType::Editable => None,
        _ => Some(valid_filter),
    };

    let mut files = PairingHeap::new();
    for res in rdr.deserialize() {
        let finfo: FileInfo = res.unwrap();
        if contains_all_substrings(&finfo.filename) {
            match pre_filter {
                Some(pre_f) if pre_f(&finfo.filename) => files.push(frecency_and_file_info(finfo)),
                None => files.push(frecency_and_file_info(finfo)),
                Some(_) => (), //pre_filter effectively filters (by not adding)
                }
        }
    }

    while let Some((_, FileInfo{filename, ..})) = files.peek() {
        if let Some(post_fi) = post_filter {
            if post_fi(filename) {
                Command::new(&prog).arg(filename).spawn().expect("Failed to execute program with relevant file");
                break;
            }
        } else {
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
        md.is_file() && !md.permissions().readonly() && md.len() <  EDITABLE_FILE_MAX_SIZE_CRITERION
            //NOTE: this criterion is ok for now. later we must find some features like "bigram patterns" of first 100 characters... that is not as time-consuming as a `$(file thisfile)` invocation
    } else {
        false
    }
}

fn valid_filter(filename: &str) -> bool {
    Path::new(filename).exists()
}
