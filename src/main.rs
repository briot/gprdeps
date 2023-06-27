use std::path::{Path, PathBuf};
use std::ffi::OsStr;

pub mod lexer;
pub mod scanner;

pub fn find_gpr_files(path: &Path, list_of_files: &mut Vec<PathBuf>) {
    if let Ok(iter) = std::fs::read_dir(path) {
        for entry in iter {
            if let Ok(e) = entry {
                let path = e.path();
                match path.extension().and_then(OsStr::to_str) {
                    Some("gpr") => list_of_files.push(path),
                    _           => {
                        if let Ok(meta) = std::fs::metadata(&path) {
                            if meta.is_dir() {
                                find_gpr_files(&path, list_of_files);
                            }
                        }
                    },
                }
            }
        }
    }
}

pub fn parse_gpr_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = std::fs::read_to_string(path)?;
    let mut lex = lexer::Lexer::new(&buffer);
    let mut scan = scanner::Scanner::new();
    scan.parse(&mut lex)?;
    Ok(())
}

pub fn parse_all(list_of_gpr: &Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for gpr in list_of_gpr {
        println!("Parsing {:?}", gpr);
        parse_gpr_file(&gpr)?;
    }
    Ok(())
}

fn main() {
    let mut list_of_gpr: Vec<PathBuf> = vec![];
    find_gpr_files(Path::new("/home/briot/dbc/deepblue"), &mut list_of_gpr);
    match parse_all(&list_of_gpr) {
        Err(e) => println!("ERROR: {}", e),
        _      => {}
    }
}
