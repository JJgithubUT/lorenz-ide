//! Pruebas de carga de una descripcion desde TOML (RF-ARQ-001, RF-ARQ-002, RF-ARQ-013).

use lorenz_arch::modelo::*;
use lorenz_arch::{cargar, ErrorArquitectura};

/// Descripcion minima valida, base de las pruebas que alteran una sola cosa.
fn minima() -> String {
    r#"
[maquina]
nombre = "min"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

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
"#
    .to_string()
}

#[test]
fn carga_una_descripcion_minima() {
    let m = cargar(&minima()).expect("deberia cargar");
    assert_eq!(m.nombre, "min");
    assert_eq!(m.bits_palabra, 8);
    assert_eq!(m.bits_direccion, 16);
}

#[test]
fn carga_la_maquina_de_ejemplo() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).expect("la maquina de ejemplo debe ser valida");
    assert_eq!(m.nombre, "ejemplo-8");
    assert_eq!(m.registros.len(), 7);
    assert_eq!(m.instrucciones.len(), 8);
    assert_eq!(m.regiones.len(), 3);
    assert_eq!(m.dispositivos.len(), 2);
}

#[test]
fn rechaza_una_version_mayor_desconocida() {
    let texto = minima().replace(r#"formato = "0.1""#, r#"formato = "1.0""#);
    let e = cargar(&texto).unwrap_err();
    assert!(matches!(e, ErrorArquitectura::VersionFormato { .. }));
}

#[test]
fn acepta_una_version_menor_distinta() {
    let texto = minima().replace(r#"formato = "0.1""#, r#"formato = "0.9""#);
    assert!(cargar(&texto).is_ok());
}

#[test]
fn rechaza_toml_mal_formado() {
    let e = cargar("[maquina\nnombre =").unwrap_err();
    assert!(matches!(e, ErrorArquitectura::Sintaxis(_)));
}

#[test]
fn el_endianness_por_defecto_es_little() {
    let m = cargar(&minima()).unwrap();
    assert_eq!(m.endianness, Endianness::Little);
}

#[test]
fn lee_el_endianness_declarado() {
    let texto = minima().replace("arranque = 0x0000", "arranque = 0x0000\nendianness = \"big\"");
    assert_eq!(cargar(&texto).unwrap().endianness, Endianness::Big);
}

#[test]
fn la_pila_por_defecto_es_descendente() {
    assert_eq!(cargar(&minima()).unwrap().pila, SentidoPila::Descendente);
}

#[test]
fn lee_los_roles_de_registro() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    assert_eq!(m.registro_con_rol(RolRegistro::ContadorPrograma).unwrap().nombre, "PC");
    assert_eq!(m.registro_con_rol(RolRegistro::PunteroPila).unwrap().nombre, "SP");
    assert_eq!(m.registro_con_rol(RolRegistro::Banderas).unwrap().nombre, "F");
}

#[test]
fn lee_las_banderas_con_su_bit() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    assert_eq!(m.bandera("Z").unwrap().bit, 0);
    assert_eq!(m.bandera("C").unwrap().bit, 1);
}

#[test]
fn el_coste_fijo_se_lee_como_numero() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    assert_eq!(m.instruccion("ADD").unwrap().ciclos, Ciclos::Fijo(2));
}

#[test]
fn el_coste_variable_se_lee_como_tabla() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    let ciclos = m.instruccion("JZ").unwrap().ciclos;
    assert_eq!(ciclos, Ciclos::SegunSalto { tomado: 3, no_tomado: 2 });
    assert_eq!(ciclos.peor_caso(), 3);
}

#[test]
fn la_sintaxis_ausente_toma_el_mnemonico() {
    let m = cargar(&minima()).unwrap();
    assert_eq!(m.instruccion("HLT").unwrap().sintaxis, "HLT");
}

#[test]
fn lee_los_operandos_de_registro_con_su_lista() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    let mov = m.instruccion("MOV").unwrap();
    match &mov.operandos[0].tipo {
        TipoOperando::Registro { permitidos } => assert_eq!(permitidos.len(), 4),
        otro => panic!("se esperaba un operando de registro, hay {otro:?}"),
    }
}

#[test]
fn lee_el_rango_de_un_inmediato() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    let ldi = m.instruccion("LDI").unwrap();
    let imm = ldi.operandos.iter().find(|o| o.nombre == "imm").unwrap();
    match imm.tipo {
        TipoOperando::Inmediato { signo, rango } => {
            assert!(!signo);
            assert_eq!(rango, (0, 255));
        }
        _ => panic!("imm deberia ser inmediato"),
    }
}

#[test]
fn lee_los_permisos_de_las_regiones() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    let rom = m.regiones.iter().find(|r| r.nombre == "rom").unwrap();
    assert!(rom.acceso.lectura && !rom.acceso.escritura);
}

#[test]
fn lee_los_puertos_de_un_dispositivo() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    let teclado = m.dispositivo("teclado").unwrap();
    assert_eq!(teclado.puertos.get("datos"), Some(&1));
    assert_eq!(teclado.interrupcion, Some(1));
}

#[test]
fn lee_los_vectores_de_interrupcion() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    let int = m.interrupciones.as_ref().unwrap();
    assert_eq!(int.vectores.get(&1), Some(&0xEFF2));
    assert!(!int.habilitadas_al_arrancar);
}

#[test]
fn una_codificacion_invalida_detiene_la_carga() {
    let texto = minima().replace(r#"codigo = "1111 ....""#, r#"codigo = "1111 rd""#);
    let e = cargar(&texto).unwrap_err();
    assert!(matches!(e, ErrorArquitectura::Codificacion { .. }));
}

#[test]
fn la_region_calcula_su_tamano() {
    let m = cargar(&minima()).unwrap();
    assert_eq!(m.regiones[0].tamano(), 0x10000);
}

#[test]
fn la_maquina_calcula_sus_maximos() {
    let m = cargar(&minima()).unwrap();
    assert_eq!(m.direccion_maxima(), 0xFFFF);
    assert_eq!(m.palabra_maxima(), 0xFF);
}
