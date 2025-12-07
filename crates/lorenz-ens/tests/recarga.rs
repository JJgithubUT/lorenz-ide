//! Recarga de la descripcion de arquitectura sin reiniciar (RF-ENS-030).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use lorenz_ens::ensamblador::Fuente;
use lorenz_ens::recarga::{Recarga, Recargable};

static CONTADOR: AtomicU32 = AtomicU32::new(0);

const BASE: &str = r#"
[maquina]
nombre = "m"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[instruccion]]
mnemonico = "NOP"
sintaxis = "NOP"
codigo = "0000 0000"

[[instruccion]]
mnemonico = "HLT"
sintaxis = "HLT"
codigo = "1111 1111"
semantica = "detener"

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
"#;

const CON_INC: &str = r#"
[[instruccion]]
mnemonico = "INC"
sintaxis = "INC"
codigo = "0001 0000"
"#;

fn archivo(texto: &str) -> PathBuf {
    let n = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("lorenz-recarga-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join("maquina.toml");
    std::fs::write(&ruta, texto).unwrap();
    ruta
}

fn ensambla(r: &Recargable, texto: &str) -> bool {
    r.ensamblador()
        .unwrap()
        .ensamblar(&Fuente::nueva("t.s", texto))
        .is_ok()
}

#[test]
fn sin_tocar_el_archivo_no_hay_cambios() {
    let ruta = archivo(BASE);
    let mut r = Recargable::abrir(&ruta).unwrap();
    assert!(matches!(r.recargar(), Recarga::SinCambios));
    assert_eq!(r.version(), 1);
}

#[test]
fn una_instruccion_nueva_aparece_sin_reiniciar() {
    let ruta = archivo(BASE);
    let mut r = Recargable::abrir(&ruta).unwrap();
    assert!(!ensambla(&r, "INC"));
    std::fs::write(&ruta, format!("{BASE}{CON_INC}")).unwrap();
    assert!(matches!(r.recargar(), Recarga::Recargada));
    assert!(ensambla(&r, "INC"));
    assert_eq!(r.version(), 2);
}

#[test]
fn una_descripcion_invalida_se_rechaza_y_se_conserva_la_anterior() {
    let ruta = archivo(&format!("{BASE}{CON_INC}"));
    let mut r = Recargable::abrir(&ruta).unwrap();
    std::fs::write(&ruta, "esto no es toml").unwrap();
    assert!(matches!(r.recargar(), Recarga::Rechazada(_)));
    assert!(ensambla(&r, "INC"));
    assert_eq!(r.version(), 1);
}

#[test]
fn el_mismo_error_no_se_repite_hasta_que_el_archivo_cambie() {
    let ruta = archivo(BASE);
    let mut r = Recargable::abrir(&ruta).unwrap();
    std::fs::write(&ruta, "[maquina").unwrap();
    assert!(matches!(r.recargar(), Recarga::Rechazada(_)));
    assert!(matches!(r.recargar(), Recarga::SinCambios));
    std::fs::write(&ruta, format!("{BASE}{CON_INC}")).unwrap();
    assert!(matches!(r.recargar(), Recarga::Recargada));
}

#[test]
fn un_archivo_borrado_se_rechaza() {
    let ruta = archivo(BASE);
    let mut r = Recargable::abrir(&ruta).unwrap();
    std::fs::remove_file(&ruta).unwrap();
    assert!(matches!(r.recargar(), Recarga::Rechazada(_)));
    assert!(ensambla(&r, "NOP"));
}

#[test]
fn abrir_un_archivo_invalido_es_un_error() {
    assert!(Recargable::abrir(archivo("nada")).is_err());
}
