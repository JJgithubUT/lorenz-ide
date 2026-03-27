//! Los fallos de validacion dicen en que linea del archivo corregirlos (RF-ARQ-034).

use lorenz_arch::{cargar, ErrorArquitectura, FalloValidacion, Regla};

const BASE: &str = r#"[maquina]
nombre = "prueba"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[flags]
Z = { bit = 0, descripcion = "cero" }

[[registro]]
nombre = "A"
bits = 8
rol = "general"

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[instruccion]]
mnemonico = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
acceso = "rw"
"#;

fn fallos(texto: &str) -> Vec<FalloValidacion> {
    match cargar(texto) {
        Err(ErrorArquitectura::Validacion(f)) => f,
        otro => panic!("se esperaban fallos de validacion: {otro:?}"),
    }
}

fn linea_de(texto: &str, buscado: &str) -> usize {
    texto.lines().position(|l| l.contains(buscado)).unwrap() + 1
}

#[test]
fn un_fallo_de_instruccion_apunta_a_su_mnemonico() {
    let texto = BASE.replace(r#"codigo = "1111 ....""#, r#"codigo = "1111 ..""#);
    let f = fallos(&texto);
    assert_eq!(
        f[0].posicion.unwrap().linea,
        linea_de(&texto, r#"mnemonico = "HLT""#)
    );
}

#[test]
fn un_nombre_repetido_apunta_a_la_segunda_declaracion() {
    let texto = BASE.replace(
        "[[instruccion]]",
        "[[registro]]\nnombre = \"A\"\nbits = 8\nrol = \"general\"\n\n[[instruccion]]",
    );
    let f = fallos(&texto);
    let segunda = texto
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains(r#"nombre = "A""#))
        .map(|(i, _)| i + 1)
        .last()
        .unwrap();
    assert!(f.iter().any(|x| x.posicion.unwrap().linea == segunda));
}

#[test]
fn un_arranque_fuera_de_region_apunta_a_la_clave() {
    let texto = BASE
        .replace("hasta = 0xFFFF", "hasta = 0x00FF")
        .replace("arranque = 0x0000", "arranque = 0x4000");
    let f = fallos(&texto);
    let del_arranque = f
        .iter()
        .find(|x| x.regla == Regla::ArranqueFueraDeRegion)
        .expect("debe haber un fallo de arranque");
    assert_eq!(
        del_arranque.posicion.unwrap().linea,
        linea_de(&texto, "arranque")
    );
}

#[test]
fn el_mensaje_incluye_linea_y_columna() {
    let texto = BASE.replace(r#"codigo = "1111 ....""#, r#"codigo = "1111 ..""#);
    let e = cargar(&texto).unwrap_err().to_string();
    assert!(
        e.contains(&format!("({}:1)", linea_de(&texto, "mnemonico"))),
        "{e}"
    );
}

#[test]
fn todo_fallo_lleva_posicion() {
    let texto = BASE.replace("rol = \"pc\"", "rol = \"general\"");
    assert!(fallos(&texto).iter().all(|f| f.posicion.is_some()));
}
