//! Herencia entre descripciones (RF-ARQ-035).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use lorenz_arch::{cadena_de_herencia, cargar, cargar_archivo, Ciclos, ErrorArquitectura};

static N: AtomicU32 = AtomicU32::new(0);

const PADRE: &str = r#"
[maquina]
nombre = "base"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16

[flags]
Z = { bit = 0, descripcion = "cero" }

[[registro]]
nombre = "A"
bits = 8

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[registro]]
nombre = "F"
bits = 8
rol = "flags"

[[instruccion]]
mnemonico = "NOP"
sintaxis = "NOP"
codigo = "0000 0000"
ciclos = 1

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

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-herencia-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn escribir(dir: &Path, nombre: &str, texto: &str) -> PathBuf {
    let r = dir.join(nombre);
    std::fs::write(&r, texto).unwrap();
    r
}

fn hijo(cuerpo_maquina: &str, resto: &str) -> String {
    format!("[maquina]\nhereda = \"base.toml\"\n{cuerpo_maquina}\n{resto}")
}

#[test]
fn un_hijo_sin_cambios_es_igual_al_padre() {
    let d = carpeta();
    escribir(&d, "base.toml", PADRE);
    let h = escribir(&d, "hijo.toml", &hijo("", ""));
    let m = cargar_archivo(&h).unwrap();
    let p = cargar(PADRE).unwrap();
    assert_eq!(m.instrucciones, p.instrucciones);
    assert_eq!(m.registros, p.registros);
    assert_eq!(m.nombre, "base");
}

#[test]
fn las_claves_de_maquina_del_hijo_mandan() {
    let d = carpeta();
    escribir(&d, "base.toml", PADRE);
    let h = escribir(&d, "hijo.toml", &hijo("nombre = \"derivada\"", ""));
    assert_eq!(cargar_archivo(&h).unwrap().nombre, "derivada");
}

#[test]
fn un_elemento_nuevo_se_anade_al_final() {
    let d = carpeta();
    escribir(&d, "base.toml", PADRE);
    let h = escribir(
        &d,
        "hijo.toml",
        &hijo(
            "",
            "[[instruccion]]\nmnemonico = \"INC\"\nsintaxis = \"INC\"\ncodigo = \"0001 0000\"\n",
        ),
    );
    let m = cargar_archivo(&h).unwrap();
    assert_eq!(m.instrucciones.len(), 3);
    assert_eq!(m.instrucciones[2].mnemonico, "INC");
}

#[test]
fn un_elemento_con_el_mismo_nombre_sustituye_al_del_padre_en_su_sitio() {
    let d = carpeta();
    escribir(&d, "base.toml", PADRE);
    let h = escribir(
        &d,
        "hijo.toml",
        &hijo(
            "",
            "[[instruccion]]\nmnemonico = \"NOP\"\nsintaxis = \"NOP\"\ncodigo = \"0000 0000\"\nciclos = 4\n",
        ),
    );
    let m = cargar_archivo(&h).unwrap();
    assert_eq!(m.instrucciones.len(), 2);
    assert_eq!(m.instrucciones[0].mnemonico, "NOP");
    assert_eq!(m.instrucciones[0].ciclos, Ciclos::Fijo(4));
}

#[test]
fn eliminar_quita_el_elemento_del_padre() {
    let d = carpeta();
    escribir(&d, "base.toml", PADRE);
    let h = escribir(
        &d,
        "hijo.toml",
        &hijo(
            "",
            "[[instruccion]]\nmnemonico = \"NOP\"\neliminar = true\n",
        ),
    );
    let m = cargar_archivo(&h).unwrap();
    assert!(m.instruccion("NOP").is_none());
    assert!(m.instruccion("HLT").is_some());
}

#[test]
fn las_banderas_se_mezclan_por_nombre() {
    let d = carpeta();
    escribir(&d, "base.toml", PADRE);
    let h = escribir(
        &d,
        "hijo.toml",
        &hijo("", "[flags]\nC = { bit = 1, descripcion = \"acarreo\" }\n"),
    );
    let m = cargar_archivo(&h).unwrap();
    assert!(m.bandera("Z").is_some());
    assert!(m.bandera("C").is_some());
}

#[test]
fn la_herencia_se_encadena() {
    let d = carpeta();
    escribir(&d, "base.toml", PADRE);
    escribir(
        &d,
        "medio.toml",
        &hijo(
            "nombre = \"medio\"",
            "[[registro]]\nnombre = \"B\"\nbits = 8\n",
        ),
    );
    let nieto = escribir(
        &d,
        "nieto.toml",
        "[maquina]\nhereda = \"medio.toml\"\nnombre = \"nieto\"\n",
    );
    let m = cargar_archivo(&nieto).unwrap();
    assert_eq!(m.nombre, "nieto");
    assert!(m.registro("B").is_some());
    assert!(m.registro("A").is_some());
    let cadena = cadena_de_herencia(&nieto);
    assert_eq!(cadena.len(), 3);
}

#[test]
fn un_ciclo_se_detecta() {
    let d = carpeta();
    let a = escribir(&d, "a.toml", "[maquina]\nhereda = \"b.toml\"\n");
    escribir(&d, "b.toml", "[maquina]\nhereda = \"a.toml\"\n");
    let e = cargar_archivo(&a).unwrap_err();
    assert!(
        matches!(e, ErrorArquitectura::Herencia(ref m) if m.contains("ciclo")),
        "{e}"
    );
}

#[test]
fn un_padre_que_no_existe_da_un_error_claro() {
    let d = carpeta();
    let h = escribir(&d, "hijo.toml", &hijo("", ""));
    let e = cargar_archivo(&h).unwrap_err();
    assert!(e.to_string().contains("base.toml"), "{e}");
}

#[test]
fn cargar_desde_texto_no_puede_heredar() {
    let e = cargar(&hijo("", "")).unwrap_err();
    assert!(matches!(e, ErrorArquitectura::Herencia(_)), "{e}");
}

#[test]
fn una_cadena_demasiado_larga_se_rechaza() {
    let d = carpeta();
    escribir(&d, "n0.toml", PADRE);
    for i in 1..=10 {
        escribir(
            &d,
            &format!("n{i}.toml"),
            &format!("[maquina]\nhereda = \"n{}.toml\"\n", i - 1),
        );
    }
    let e = cargar_archivo(d.join("n10.toml")).unwrap_err();
    assert!(e.to_string().contains("niveles"), "{e}");
}

#[test]
fn el_resultado_se_valida_como_cualquier_descripcion() {
    let d = carpeta();
    escribir(&d, "base.toml", PADRE);
    // Una instruccion que repite la codificacion de NOP rompe la regla de ambiguedad.
    let h = escribir(
        &d,
        "hijo.toml",
        &hijo(
            "",
            "[[instruccion]]\nmnemonico = \"VACIO\"\nsintaxis = \"VACIO\"\ncodigo = \"0000 0000\"\n",
        ),
    );
    assert!(matches!(
        cargar_archivo(&h).unwrap_err(),
        ErrorArquitectura::Validacion(_)
    ));
}

#[test]
fn hereda_debe_ser_un_texto() {
    let e = cargar("[maquina]\nhereda = 3\n").unwrap_err();
    assert!(matches!(e, ErrorArquitectura::Herencia(_)), "{e}");
}
