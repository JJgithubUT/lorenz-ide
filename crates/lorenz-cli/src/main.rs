use std::io::{stderr, stdout};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let codigo = lorenz_cli::ejecutar_cli(&args, &mut stdout().lock(), &mut stderr().lock());
    std::process::exit(codigo);
}
