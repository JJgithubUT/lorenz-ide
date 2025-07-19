//! Pruebas de las reglas de validacion.
//!
//! Cada prueba rompe exactamente una regla sobre una descripcion por lo demas valida, y comprueba
//! que el cargador la detecta citando su codigo.

use lorenz_arch::{cargar, ErrorArquitectura, Regla};

fn base() -> String {
    r#"
[maquina]
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

[[registro]]
nombre = "F"
bits = 8
rol = "flags"

[[instruccion]]
mnemonico = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
acceso = "rw"
"#
    .to_string()
}

/// Carga esperando fallos y devuelve los codigos de regla incumplidos.
fn codigos(texto: &str) -> Vec<&'static str> {
    match cargar(texto) {
        Ok(_) => Vec::new(),
        Err(ErrorArquitectura::Validacion(fallos)) => {
            fallos.iter().map(|f| f.regla.codigo()).collect()
        }
        Err(e) => panic!("se esperaba un fallo de validacion, llego: {e}"),
    }
}

#[test]
fn la_descripcion_base_es_valida() {
    assert!(cargar(&base()).is_ok());
}

#[test]
fn detecta_una_palabra_que_no_suma_el_ancho() {
    let texto = base().replace(r#"codigo = "1111 ....""#, r#"codigo = "1111 ..""#);
    assert!(codigos(&texto).contains(&Regla::SumaDeBits.codigo()));
}

#[test]
fn detecta_dos_instrucciones_ambiguas() {
    let texto = base()
        + r#"
[[instruccion]]
mnemonico = "STOP"
codigo = "1111 ...."
semantica = "detener"
"#;
    assert!(codigos(&texto).contains(&Regla::CodificacionAmbigua.codigo()));
}

#[test]
fn acepta_dos_instrucciones_con_opcode_distinto() {
    let texto = base()
        + r#"
[[instruccion]]
mnemonico = "NOP"
codigo = "0000 ...."
semantica = ""
"#;
    assert!(cargar(&texto).is_ok());
}

#[test]
fn detecta_un_campo_sin_operando_declarado() {
    let texto = base().replace(r#"codigo = "1111 ....""#, r#"codigo = "1111 rd:4""#);
    assert!(codigos(&texto).contains(&Regla::CampoSinOperando.codigo()));
}

#[test]
fn detecta_un_operando_que_no_esta_en_la_codificacion() {
    let texto = base()
        + r#"
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A"]
"#;
    assert!(codigos(&texto).contains(&Regla::CampoSinOperando.codigo()));
}

#[test]
fn detecta_un_operando_que_admite_un_registro_inexistente() {
    let texto = base().replace(
        r#"codigo = "1111 ....""#,
        r#"codigo = "1111 rd:2 .."
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "Z9"]"#,
    );
    assert!(codigos(&texto).contains(&Regla::CampoSinOperando.codigo()));
}

#[test]
fn detecta_mas_registros_de_los_que_codifica_el_campo() {
    let texto = base().replace(
        r#"codigo = "1111 ....""#,
        r#"codigo = "1111 rd:1 ..."
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "PC", "F"]"#,
    );
    assert!(codigos(&texto).contains(&Regla::CampoSinOperando.codigo()));
}

#[test]
fn detecta_la_falta_de_contador_de_programa() {
    let texto = base().replace(r#"rol = "pc""#, r#"rol = "general""#);
    assert!(codigos(&texto).contains(&Regla::RolUnico.codigo()));
}

#[test]
fn detecta_dos_contadores_de_programa() {
    let texto = base()
        + r#"
[[registro]]
nombre = "PC2"
bits = 16
rol = "pc"
"#;
    assert!(codigos(&texto).contains(&Regla::RolUnico.codigo()));
}

#[test]
fn detecta_banderas_sin_registro_de_banderas() {
    let texto = base().replace(
        r#"[[registro]]
nombre = "F"
bits = 8
rol = "flags""#,
        "",
    );
    assert!(codigos(&texto).contains(&Regla::RolUnico.codigo()));
}

#[test]
fn detecta_dos_registros_con_el_mismo_nombre() {
    let texto = base()
        + r#"
[[registro]]
nombre = "A"
bits = 8
rol = "general"
"#;
    assert!(codigos(&texto).contains(&Regla::NombreDuplicado.codigo()));
}

#[test]
fn detecta_regiones_solapadas() {
    let texto = base()
        + r#"
[[region]]
nombre = "otra"
desde = 0x8000
hasta = 0x8FFF
acceso = "rw"
"#;
    assert!(codigos(&texto).contains(&Regla::RegionSolapada.codigo()));
}

#[test]
fn acepta_regiones_contiguas_sin_solape() {
    let texto = base().replace("hasta = 0xFFFF", "hasta = 0x7FFF")
        + r#"
[[region]]
nombre = "alta"
desde = 0x8000
hasta = 0xFFFF
acceso = "rw"
"#;
    assert!(cargar(&texto).is_ok());
}

#[test]
fn detecta_una_region_invertida() {
    let texto = base().replace("desde = 0x0000\nhasta = 0xFFFF", "desde = 0xF000\nhasta = 0x0FFF");
    assert!(codigos(&texto).contains(&Regla::ValorFueraDeRango.codigo()));
}

#[test]
fn detecta_una_region_fuera_del_espacio_direccionable() {
    let texto = base().replace("bits_direccion = 16", "bits_direccion = 8");
    assert!(codigos(&texto).contains(&Regla::ValorFueraDeRango.codigo()));
}

#[test]
fn detecta_un_arranque_fuera_de_toda_region() {
    let texto = base()
        .replace("arranque = 0x0000", "arranque = 0x1000")
        .replace("hasta = 0xFFFF", "hasta = 0x0FFF");
    assert!(codigos(&texto).contains(&Regla::ArranqueFueraDeRegion.codigo()));
}

#[test]
fn detecta_una_region_que_cita_un_dispositivo_inexistente() {
    let texto = base() + "dispositivo = \"pantalla\"\n";
    assert!(codigos(&texto).contains(&Regla::DispositivoInexistente.codigo()));
}

#[test]
fn detecta_una_bandera_inexistente_en_afecta() {
    let texto = base().replace(r#"semantica = "detener""#, "semantica = \"detener\"\nafecta = [\"N\"]");
    assert!(codigos(&texto).contains(&Regla::BanderaInexistente.codigo()));
}

#[test]
fn detecta_un_ancho_de_palabra_imposible() {
    let texto = base().replace("bits_palabra = 8", "bits_palabra = 3");
    assert!(codigos(&texto).contains(&Regla::ValorFueraDeRango.codigo()));
}

#[test]
fn detecta_un_valor_inicial_que_no_cabe_en_el_registro() {
    let texto = base().replace(
        r#"nombre = "A"
bits = 8
rol = "general""#,
        r#"nombre = "A"
bits = 8
rol = "general"
inicial = 0x1FF"#,
    );
    assert!(codigos(&texto).contains(&Regla::ValorFueraDeRango.codigo()));
}

#[test]
fn detecta_una_bandera_fuera_del_registro_de_banderas() {
    let texto = base().replace("Z = { bit = 0", "Z = { bit = 9");
    assert!(codigos(&texto).contains(&Regla::ValorFueraDeRango.codigo()));
}

#[test]
fn detecta_dos_banderas_en_el_mismo_bit() {
    let texto = base().replace(
        r#"Z = { bit = 0, descripcion = "cero" }"#,
        "Z = { bit = 0, descripcion = \"cero\" }\nC = { bit = 0, descripcion = \"acarreo\" }",
    );
    assert!(codigos(&texto).contains(&Regla::NombreDuplicado.codigo()));
}

#[test]
fn detecta_un_dispositivo_cuya_linea_no_tiene_vector() {
    let texto = base()
        + r#"
[maquina.interrupciones]
vectores = { 0 = 0xFF00 }

[[dispositivo]]
nombre = "teclado"
tipo = "teclado"
interrupcion = 3
"#;
    assert!(codigos(&texto).contains(&Regla::DispositivoInexistente.codigo()));
}

#[test]
fn acumula_varios_fallos_en_una_sola_carga() {
    let texto = base()
        .replace(r#"codigo = "1111 ....""#, r#"codigo = "1111 ..""#)
        .replace(r#"rol = "pc""#, r#"rol = "general""#);
    let fallos = codigos(&texto);
    assert!(fallos.len() >= 2, "se esperaban al menos dos fallos, hubo {fallos:?}");
    assert!(fallos.contains(&Regla::SumaDeBits.codigo()));
    assert!(fallos.contains(&Regla::RolUnico.codigo()));
}

#[test]
fn cada_regla_cita_su_requisito_de_la_ers() {
    for regla in [
        Regla::SumaDeBits,
        Regla::CodificacionAmbigua,
        Regla::CampoSinOperando,
        Regla::RolUnico,
        Regla::RegionSolapada,
        Regla::ArranqueFueraDeRegion,
        Regla::DispositivoInexistente,
        Regla::BanderaInexistente,
        Regla::NombreDuplicado,
        Regla::ValorFueraDeRango,
    ] {
        assert!(regla.requisito().starts_with("RF-ARQ-"), "{regla:?} no cita requisito");
    }
}
