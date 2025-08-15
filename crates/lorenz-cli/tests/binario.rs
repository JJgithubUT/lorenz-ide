//! Pruebas del binario `lorenz` como proceso: comprueban que el codigo de salida llega al sistema.

use std::process::Command;

fn lorenz() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lorenz"))
}

fn ejemplo(ruta: &str) -> String {
    format!(
        "{}/../../ejemplos/ejemplo-8/{ruta}",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn el_proceso_devuelve_cero_cuando_la_maquina_se_detiene() {
    let s = lorenz()
        .args([
            "ejecutar",
            &ejemplo("maquina.toml"),
            &ejemplo("programas/suma.hex"),
        ])
        .output()
        .unwrap();
    assert_eq!(s.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&s.stdout).contains("A=0x2c"));
}

#[test]
fn el_proceso_devuelve_tres_al_alcanzar_el_limite() {
    let s = lorenz()
        .args([
            "ejecutar",
            &ejemplo("maquina.toml"),
            &ejemplo("programas/bucle.hex"),
            "--limite",
            "10",
        ])
        .output()
        .unwrap();
    assert_eq!(s.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&s.stderr).contains("limite"));
}

#[test]
fn el_proceso_devuelve_dos_con_argumentos_incorrectos() {
    let s = lorenz().arg("bailar").output().unwrap();
    assert_eq!(s.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&s.stderr).contains("bailar"));
}

#[test]
fn el_proceso_sin_argumentos_explica_como_pedir_ayuda() {
    let s = lorenz().output().unwrap();
    assert_eq!(s.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&s.stderr).contains("ayuda"));
}
