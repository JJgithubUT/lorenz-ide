use std::io::{stderr, stdin, stdout};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let codigo = lorenz_cli::ejecutar_cli_con_entrada(
        &args,
        &mut stdin().lock(),
        &mut stdout().lock(),
        &mut stderr().lock(),
    );
    std::process::exit(codigo);
}
