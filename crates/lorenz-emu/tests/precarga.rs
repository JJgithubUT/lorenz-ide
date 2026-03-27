//! Precargar regiones desde un archivo (RF-ARQ-029).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_emu::programa::precargar_regiones;
use lorenz_emu::Emulador;

static N: AtomicUsize = AtomicUsize::new(0);

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-precarga-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn emulador_con(contenido: &str) -> Emulador {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let t = t.replacen(
        "nombre = \"rom\"",
        &format!("nombre = \"rom\"\ncontenido = \"{contenido}\""),
        1,
    );
    Emulador::nuevo(lorenz_arch::cargar(&t).unwrap()).unwrap()
}

fn leer(e: &Emulador, d: u64, n: usize) -> Vec<u64> {
    e.memoria().volcar(d, n).unwrap()
}

#[test]
fn el_contenido_se_escribe_al_inicio_de_la_region_aunque_sea_de_solo_lectura() {
    let d = carpeta();
    std::fs::write(d.join("rom.hex"), "# arranque\n11 22 33\n").unwrap();
    let mut e = emulador_con("rom.hex");
    assert_eq!(precargar_regiones(&mut e, &d), Ok(1));
    assert_eq!(leer(&e, 0, 4), vec![0x11, 0x22, 0x33, 0]);
}

#[test]
fn sin_contenido_declarado_no_hace_nada() {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let mut e = Emulador::nuevo(lorenz_arch::cargar(&t).unwrap()).unwrap();
    assert_eq!(precargar_regiones(&mut e, Path::new(".")), Ok(0));
}

#[test]
fn un_archivo_que_falta_se_explica() {
    let mut e = emulador_con("no-existe.hex");
    let m = precargar_regiones(&mut e, &carpeta()).unwrap_err();
    assert!(m.contains("rom") && m.contains("no-existe.hex"), "{m}");
}

#[test]
fn un_contenido_mas_grande_que_la_region_se_rechaza() {
    let d = carpeta();
    let texto = "00 ".repeat(0x2001);
    std::fs::write(d.join("grande.hex"), texto).unwrap();
    let mut e = emulador_con("grande.hex");
    let m = precargar_regiones(&mut e, &d).unwrap_err();
    assert!(m.contains("8193") && m.contains("8192"), "{m}");
}

#[test]
fn una_palabra_invalida_dice_en_que_region_y_archivo() {
    let d = carpeta();
    std::fs::write(d.join("mal.hex"), "11 zz").unwrap();
    let mut e = emulador_con("mal.hex");
    let m = precargar_regiones(&mut e, &d).unwrap_err();
    assert!(
        m.contains("rom") && m.contains("mal.hex") && m.contains("zz"),
        "{m}"
    );
}
