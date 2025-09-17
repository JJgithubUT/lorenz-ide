//! Validacion de la sintaxis de cada instruccion (RF-ARQ-013).

use lorenz_arch::{cargar, ErrorArquitectura, Regla};

fn maquina(instruccion: &str) -> String {
    format!(
        r#"
[maquina]
nombre = "s"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[[registro]]
nombre = "A"
bits = 8
rol = "general"

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
acceso = "rw"

{instruccion}
"#
    )
}

const MOV: &str = r#"
[[instruccion]]
mnemonico = "MOV"
%SINTAXIS%
codigo = "0001 rd:2 rs:2"
semantica = "rd = rs"
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "A", "A", "A"]
  [[instruccion.operando]]
  nombre = "rs"
  tipo = "registro"
  de = ["A", "A", "A", "A"]
"#;

fn con_sintaxis(linea: &str) -> String {
    maquina(&MOV.replace("%SINTAXIS%", linea))
}

fn reglas(texto: &str) -> Vec<Regla> {
    match cargar(texto) {
        Ok(_) => Vec::new(),
        Err(ErrorArquitectura::Validacion(f)) => f.into_iter().map(|x| x.regla).collect(),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn una_sintaxis_correcta_carga() {
    assert!(reglas(&con_sintaxis(r#"sintaxis = "MOV {rd}, {rs}""#)).is_empty());
}

#[test]
fn sin_sintaxis_se_usa_el_mnemonico_y_los_operandos_en_orden() {
    let m = cargar(&con_sintaxis("")).unwrap();
    assert_eq!(m.instruccion("MOV").unwrap().sintaxis, "MOV {rd}, {rs}");
}

#[test]
fn una_sintaxis_con_otro_mnemonico_se_rechaza() {
    let r = reglas(&con_sintaxis(r#"sintaxis = "COPIA {rd}, {rs}""#));
    assert_eq!(r, vec![Regla::SintaxisInvalida]);
}

#[test]
fn el_mnemonico_de_la_sintaxis_no_distingue_mayusculas() {
    assert!(reglas(&con_sintaxis(r#"sintaxis = "mov {rd}, {rs}""#)).is_empty());
}

#[test]
fn un_hueco_que_no_es_operando_se_rechaza() {
    let r = reglas(&con_sintaxis(r#"sintaxis = "MOV {rd}, {rx}""#));
    // `rx` no existe y `rs` queda sin aparecer.
    assert_eq!(r, vec![Regla::SintaxisInvalida, Regla::SintaxisInvalida]);
}

#[test]
fn un_operando_ausente_de_la_sintaxis_se_rechaza() {
    let r = reglas(&con_sintaxis(r#"sintaxis = "MOV {rd}""#));
    assert_eq!(r, vec![Regla::SintaxisInvalida]);
}

#[test]
fn un_operando_repetido_se_rechaza() {
    let r = reglas(&con_sintaxis(r#"sintaxis = "MOV {rd}, {rd}""#));
    assert!(r.contains(&Regla::SintaxisInvalida));
}

#[test]
fn una_llave_sin_cerrar_se_rechaza() {
    let r = reglas(&con_sintaxis(r#"sintaxis = "MOV {rd, {rs}""#));
    assert!(r.contains(&Regla::SintaxisInvalida));
}

#[test]
fn el_codigo_y_el_requisito_de_la_regla() {
    assert_eq!(Regla::SintaxisInvalida.codigo(), "E-SINTAXIS");
    assert_eq!(Regla::SintaxisInvalida.requisito(), "RF-ARQ-013");
}
