use std::env;
use std::fs;
use std::process::Command;

#[derive(Debug)]
struct FileInfo<'a>{
    filename: &'a str,
    rank: f32,
    timestamp: u32,
}

impl FileInfo <'_> {
    pub fn new<'a>(line: &'a str) -> Result<FileInfo, &'a str>{
        let mut fields = line.split('|');
        let filename = match fields.next() {
            Some(s) => s,
            None => return Err("Unable to extract filename"),
        };
        let rank:f32 = match fields.next().unwrap().parse() {
            Ok(r) => r,
            Err(_) => return Err("Unable to parse rank as float"),
        };
        let timestamp:u32 = match fields.next().unwrap().parse() {
            Ok(ts) => ts,
            Err(_) => return Err("Unable to parse timestamp as unsigned int"),
        };
        Ok(
            FileInfo{
                filename,
                rank,
                timestamp,
            })
    }
}

fn main() {
    let home = env::var("HOME").unwrap();
    let fasd_file = format!("{}/.fasd", home);

    let contents = fs::read_to_string(fasd_file).unwrap();
    let files = contents.lines()
        .map(FileInfo::new)
        .filter_map(Result::ok)
        .filter(|info| info.filename.ends_with(".pdf"));
    for file in files{
        println!("{}", file.filename);
    }
}

fn now_ts() -> u32{
    let now = Command::new("date").arg("+%s").output().unwrap();
    let now = String::from_utf8_lossy(now.stdout.as_slice());
    let now = now.trim();
    let now: u32 = now.parse().unwrap();
    now
}
