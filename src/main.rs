use std::path::Path;

pub mod scanner;

pub fn parse_gpr_file(path: &Path) -> std::io::Result<()> {
    let buffer = std::fs::read_to_string(path)?;
    let mut scan = scanner::Scanner::new(&buffer);

    while let Some(token) = scan.next_token() {
        println!("Token = {:?}", token);
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    parse_gpr_file(Path::new("data_server.gpr"))
}
