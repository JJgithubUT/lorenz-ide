//! Pruebas de la interfaz web con Chromium. Se omiten si no hay Python con playwright o Chromium.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn banco_de_pruebas_de_la_interfaz() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let banco = repo.join("pruebas-ui/banco.py");
    let binario = PathBuf::from(env!("CARGO_BIN_EXE_lorenz"));
    let Ok(salida) = Command::new("python3").arg(&banco).arg(&binario).output() else {
        eprintln!("omitida: no hay python3");
        return;
    };
    let texto = String::from_utf8_lossy(&salida.stdout);
    match salida.status.code() {
        Some(0) => {}
        Some(77) => eprintln!("omitida: {texto}"),
        _ => panic!(
            "fallo la interfaz:\n{texto}\n{}",
            String::from_utf8_lossy(&salida.stderr)
        ),
    }
}
