use std::env;
use std::fs;
use std::process::Command;
use std::path::Path;

#[derive(Debug)]
pub struct FileInfo<'a>{
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

    let mut args = env::args().skip(1);
    let fileext = args.next().unwrap();
    let prog = args.next().unwrap();

    let contents = fs::read_to_string(fasd_file).unwrap();
    let files = contents.lines()
        .map(FileInfo::new)
        .filter_map(Result::ok)
        .filter(|info| info.filename.ends_with(&fileext) && Path::new(info.filename).exists());
    let now = now_ts();
    let orf = files
        .max_by_key(|f| (f.frecency(now) * 10000 as f32) as u32); // As there is no ordering in float...
    let orf = orf.unwrap();
    Command::new(&prog).arg(orf.filename).spawn().expect("Failed to execute program with relevant file");
}

fn now_ts() -> u32{
    let now = Command::new("date").arg("+%s").output().unwrap();
    let now = String::from_utf8_lossy(now.stdout.as_slice());
    let now = now.trim();
    let now: u32 = now.parse().unwrap();
    now
}
