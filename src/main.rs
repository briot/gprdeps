use std::path::{Path, PathBuf};
use std::ffi::OsStr;

pub mod errors;
pub mod files;
pub mod lexer;
pub mod scanner;
pub mod tokens;

pub fn find_gpr_files(path: &Path, list_of_files: &mut Vec<PathBuf>) {
    if let Ok(iter) = std::fs::read_dir(path) {
        for e in iter.flatten() {
            let path = e.path();
            match path.extension().and_then(OsStr::to_str) {
                Some("gpr") => list_of_files.push(path),
                _           => {
                    if let Ok(meta) = std::fs::metadata(&path) {
                        let name = path.as_os_str().to_str();
                        if let Some(n) = name {
                            if !n.ends_with("External/Ada_Web_Server/aws-dev")
                               && !n.ends_with("External/GNATCOLL/gnatcoll-dev")
                               && !n.ends_with("Packaging")
                               && !n.ends_with("Compiler")
                               && !n.ends_with(".dbc")
                               && meta.is_dir()
                               && !meta.is_symlink()
                            {
                                find_gpr_files(&path, list_of_files);
                            }
                        }
                    }
                },
            }
        }
    }
}

pub fn parse_gpr_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = files::File::new(path)?;
    let mut lex = lexer::Lexer::new(&file);
    let mut scan = scanner::Scanner::new(&mut lex);
    scan.parse()?;
    Ok(())
}

pub fn parse_all(list_of_gpr: &Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for gpr in list_of_gpr {
        parse_gpr_file(gpr)?;
    }
    Ok(())
}

fn main() {
    let mut list_of_gpr: Vec<PathBuf> = vec![];
    find_gpr_files(Path::new("/home/briot/dbc/deepblue"), &mut list_of_gpr);
    if let Err(e) = parse_all(&list_of_gpr) {
        println!("ERROR: {}", e);
    }
}
