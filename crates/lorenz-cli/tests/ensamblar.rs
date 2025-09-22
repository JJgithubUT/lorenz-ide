//! Las ordenes `ensamblar` y `desensamblar`, y `ejecutar` con fuentes (RF-ENS-026).

use std::path::PathBuf;

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn maquina() -> String {
    raiz()
        .join("ejemplos/ejemplo-8/maquina.toml")
        .to_string_lossy()
        .into_owned()
}

fn ejemplo(nombre: &str) -> String {
    raiz()
        .join("ejemplos/ejemplo-8/programas")
        .join(nombre)
        .to_string_lossy()
        .into_owned()
}

fn dir_temporal() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lorenz-ens-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn temporal(nombre: &str, contenido: &str) -> String {
    let ruta = dir_temporal().join(nombre);
    std::fs::write(&ruta, contenido).unwrap();
    ruta.to_string_lossy().into_owned()
}

struct Resultado {
    codigo: i32,
    salida: String,
    errores: String,
}

fn correr(args: &[&str]) -> Resultado {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut salida, mut errores) = (Vec::new(), Vec::new());
    let codigo = ejecutar_cli(&args, &mut salida, &mut errores);
    Resultado {
        codigo,
        salida: String::from_utf8(salida).unwrap(),
        errores: String::from_utf8(errores).unwrap(),
    }
}

#[test]
fn ensamblar_sin_salida_escribe_el_hexadecimal_en_pantalla() {
    let r = correr(&["ensamblar", &maquina(), &ejemplo("suma.s")]);
    assert_eq!(r.codigo, SALIDA_OK, "{}", r.errores);
    assert!(r.salida.starts_with("# origen 0x0000\n"));
    assert!(r.salida.contains("20 C8 24 64 31 F0"));
}

#[test]
fn el_hexadecimal_ensamblado_coincide_con_el_escrito_a_mano() {
    let hex = temporal("suma-ensamblada.hex", "");
    let r = correr(&["ensamblar", &maquina(), &ejemplo("suma.s"), "-o", &hex]);
    assert_eq!(r.codigo, SALIDA_OK, "{}", r.errores);
    assert!(r.salida.contains("6 palabras en 1 segmento(s)"));
    let a =
        lorenz_cli::programa::leer_palabras(&std::fs::read_to_string(&hex).unwrap(), 8).unwrap();
    let b = lorenz_cli::programa::leer_palabras(
        &std::fs::read_to_string(ejemplo("suma.hex")).unwrap(),
        8,
    )
    .unwrap();
    assert_eq!(a, b);
}

#[test]
fn ensamblar_escribe_mapa_listado_y_simbolos() {
    let mapa = temporal("datos.mapa", "");
    let lst = temporal("datos.lst", "");
    let sim = temporal("datos.sim", "");
    let r = correr(&[
        "ensamblar",
        &maquina(),
        &ejemplo("datos.s"),
        "-o",
        &temporal("datos.hex", ""),
        "--mapa",
        &mapa,
        "--listado",
        &lst,
        "--simbolos",
        &sim,
    ]);
    assert_eq!(r.codigo, SALIDA_OK, "{}", r.errores);
    assert!(std::fs::read_to_string(&mapa).unwrap().contains("datos.s:"));
    assert!(std::fs::read_to_string(&lst)
        .unwrap()
        .contains("LD  A, [valor]"));
    let s = std::fs::read_to_string(&sim).unwrap();
    assert!(s.contains("valor = 0x0100"));
    assert!(s.contains("DESTINO = 8192"));
}

#[test]
fn un_fuente_con_errores_los_lista_todos_con_posicion() {
    let f = temporal("malo.s", "LDI A, 300\nFOO\nHLT\n");
    let r = correr(&["ensamblar", &maquina(), &f]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("malo.s:2:1:"), "{}", r.errores);
    assert!(r.errores.lines().count() >= 1);
    assert!(r.salida.is_empty());
}

#[test]
fn un_fuente_inexistente_es_un_error_de_uso() {
    let r = correr(&["ensamblar", &maquina(), "no-existe.s"]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("no se pudo leer"));
}

#[test]
fn ensamblar_sin_argumentos_pide_los_que_faltan() {
    assert_eq!(correr(&["ensamblar"]).codigo, SALIDA_ERROR_USO);
    let r = correr(&["ensamblar", &maquina()]);
    assert!(r.errores.contains("<fuente>"));
    let r = correr(&["ensamblar", &maquina(), "a.s", "--nada"]);
    assert!(r.errores.contains("--nada"));
    let r = correr(&["ensamblar", &maquina(), "a.s", "-o"]);
    assert!(r.errores.contains("necesita un valor"));
}

#[test]
fn desensamblar_muestra_el_texto_de_cada_instruccion() {
    let r = correr(&["desensamblar", &maquina(), &ejemplo("suma.hex")]);
    assert_eq!(r.codigo, SALIDA_OK, "{}", r.errores);
    assert!(r.salida.contains("LDI A, 200"));
    assert!(r.salida.contains("ADD A, B"));
    assert!(r.salida.contains("HLT"));
}

#[test]
fn desensamblar_con_simbolos_usa_etiquetas() {
    let sim = temporal("bucle.sim", "[etiquetas]\ninicio = 0\n");
    let r = correr(&[
        "desensamblar",
        &maquina(),
        &ejemplo("bucle.hex"),
        "--simbolos",
        &sim,
    ]);
    assert_eq!(r.codigo, SALIDA_OK, "{}", r.errores);
    assert!(r.salida.contains("inicio:"));
}

#[test]
fn desensamblar_respeta_la_direccion_inicial() {
    let r = correr(&[
        "desensamblar",
        &maquina(),
        &ejemplo("suma.hex"),
        "--direccion",
        "0x100",
    ]);
    assert!(r.salida.contains(".org 0x0100"));
}

#[test]
fn lo_desensamblado_se_vuelve_a_ensamblar_igual() {
    let d = correr(&["desensamblar", &maquina(), &ejemplo("suma.hex")]);
    let fuente = temporal("redescubierto.s", &d.salida);
    let r = correr(&["ensamblar", &maquina(), &fuente]);
    assert_eq!(r.codigo, SALIDA_OK, "{}", r.errores);
    assert!(r.salida.contains("20 C8 24 64 31 F0"));
}

#[test]
fn ejecutar_un_fuente_da_el_mismo_resultado_que_el_hexadecimal() {
    let a = correr(&["ejecutar", &maquina(), &ejemplo("suma.s")]);
    let b = correr(&["ejecutar", &maquina(), &ejemplo("suma.hex")]);
    assert_eq!(a.codigo, SALIDA_OK, "{}", a.errores);
    assert_eq!(a.salida, b.salida);
    assert!(a.salida.contains("A=0x2c"));
}

#[test]
fn ejecutar_un_fuente_con_varios_segmentos_carga_cada_uno_en_su_direccion() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &ejemplo("datos.s"),
        "--volcar",
        "0x2000:1",
    ]);
    assert_eq!(r.codigo, SALIDA_OK, "{}", r.errores);
    assert!(r.salida.contains("5A"), "{}", r.salida);
}

#[test]
fn el_bucle_en_ensamblador_llega_al_limite() {
    let r = correr(&[
        "ejecutar",
        &maquina(),
        &ejemplo("bucle.s"),
        "--limite",
        "20",
    ]);
    assert_eq!(r.codigo, SALIDA_LIMITE);
}

#[test]
fn ejecutar_un_fuente_con_errores_no_arranca() {
    let f = temporal("roto.s", "FOO\n");
    let r = correr(&["ejecutar", &maquina(), &f]);
    assert_eq!(r.codigo, SALIDA_ERROR_USO);
    assert!(r.errores.contains("roto.s:1:1:"));
}

#[test]
fn la_ayuda_documenta_las_nuevas_ordenes() {
    let r = correr(&["ayuda"]);
    assert!(r.salida.contains("lorenz ensamblar"));
    assert!(r.salida.contains("lorenz desensamblar"));
}
