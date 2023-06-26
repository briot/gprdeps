use std::path::Path;

pub mod lexer;
pub mod scanner;

pub fn parse_gpr_file(path: &Path) {
    let buffer = std::fs::read_to_string(path);
    match buffer {
        Err(e) => {
            println!("ERROR: {}", e);
            return;
        },
        Ok(b) => {
            let mut lex = lexer::Lexer::new(&b);
            let mut scan = scanner::Scanner::new();

            match scan.parse(&mut lex) {
                Err(e) => println!("ERROR: {}", e),
                Ok(_)  => println!("SUCCESS"),
            }
        }
    };
}

fn main() {
    parse_gpr_file(Path::new("data_server.gpr"));
}
