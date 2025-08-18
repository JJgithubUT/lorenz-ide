//! Pruebas del decodificador con la maquina de ejemplo de 8 bits.

mod comun;

use comun::ejemplo_8;
use lorenz_emu::{Decodificador, ErrorEjecucion, Memoria};

fn dec() -> Decodificador {
    Decodificador::nuevo(&ejemplo_8())
}

fn indice(m: &str) -> usize {
    ejemplo_8()
        .instrucciones
        .iter()
        .position(|i| i.mnemonico == m)
        .unwrap()
}

fn valor(d: &lorenz_emu::Decodificada, nombre: &str) -> i128 {
    d.operandos
        .iter()
        .find(|o| o.nombre == nombre)
        .unwrap()
        .valor
}

fn registro(d: &lorenz_emu::Decodificada, nombre: &str) -> String {
    d.operandos
        .iter()
        .find(|o| o.nombre == nombre)
        .unwrap()
        .registro
        .clone()
        .unwrap()
}

#[test]
fn nop_ocupa_una_palabra_y_no_tiene_operandos() {
    let d = dec().decodificar_palabras(0, &[0x00]).unwrap();
    assert_eq!(d.indice, indice("NOP"));
    assert_eq!(d.longitud, 1);
    assert!(d.operandos.is_empty());
}

#[test]
fn nop_ignora_los_bits_de_relleno() {
    let d = dec().decodificar_palabras(0, &[0x0F]).unwrap();
    assert_eq!(d.indice, indice("NOP"));
}

#[test]
fn mov_decodifica_dos_registros() {
    // 0001 rd=01 rs=10  -> MOV B, C
    let d = dec().decodificar_palabras(0, &[0b0001_0110]).unwrap();
    assert_eq!(d.indice, indice("MOV"));
    assert_eq!(registro(&d, "rd"), "B");
    assert_eq!(registro(&d, "rs"), "C");
}

#[test]
fn ldi_usa_dos_palabras_y_trae_el_inmediato() {
    // 0010 rd=11 .. | imm
    let d = dec().decodificar_palabras(0, &[0b0010_1100, 0x2A]).unwrap();
    assert_eq!(d.indice, indice("LDI"));
    assert_eq!(d.longitud, 2);
    assert_eq!(registro(&d, "rd"), "D");
    assert_eq!(valor(&d, "imm"), 0x2A);
}

#[test]
fn una_direccion_repartida_en_dos_palabras_junta_primero_la_baja_en_little_endian() {
    // LD A, [0x2010]: bajo 0x10, alto 0x20
    let d = dec()
        .decodificar_palabras(0, &[0b0100_0000, 0x10, 0x20])
        .unwrap();
    assert_eq!(d.indice, indice("LD"));
    assert_eq!(d.longitud, 3);
    assert_eq!(valor(&d, "dir"), 0x2010);
}

#[test]
fn st_decodifica_el_registro_origen_y_la_direccion() {
    let d = dec()
        .decodificar_palabras(0, &[0b0101_1000, 0xFF, 0x20])
        .unwrap();
    assert_eq!(d.indice, indice("ST"));
    assert_eq!(registro(&d, "rs"), "C");
    assert_eq!(valor(&d, "dir"), 0x20FF);
}

#[test]
fn jz_y_hlt_se_reconocen() {
    assert_eq!(
        dec()
            .decodificar_palabras(0, &[0x60, 0x00, 0x00])
            .unwrap()
            .indice,
        indice("JZ")
    );
    assert_eq!(
        dec().decodificar_palabras(0, &[0xF0]).unwrap().indice,
        indice("HLT")
    );
}

#[test]
fn un_codigo_sin_instruccion_es_invalido() {
    let e = dec().decodificar_palabras(0x10, &[0x80]).unwrap_err();
    assert_eq!(
        e,
        ErrorEjecucion::InstruccionInvalida {
            direccion: 0x10,
            palabra: 0x80
        }
    );
}

#[test]
fn faltan_palabras_para_la_instruccion() {
    assert!(matches!(
        dec().decodificar_palabras(0, &[0x40, 0x10]),
        Err(ErrorEjecucion::InstruccionInvalida { .. })
    ));
}

#[test]
fn sin_palabras_no_hay_instruccion() {
    assert!(matches!(
        dec().decodificar_palabras(0, &[]),
        Err(ErrorEjecucion::InstruccionInvalida { .. })
    ));
}

#[test]
fn longitud_de_dice_cuantas_palabras_lee_cada_instruccion() {
    let d = dec();
    assert_eq!(d.longitud_de(0x00), Some(1));
    assert_eq!(d.longitud_de(0x20), Some(2));
    assert_eq!(d.longitud_de(0x40), Some(3));
    assert_eq!(d.longitud_de(0x80), None);
}

#[test]
fn decodificar_lee_las_palabras_de_memoria() {
    let m = ejemplo_8();
    let mut mem = Memoria::nueva(&m).unwrap();
    mem.cargar_bloque(0, &[0x2C, 0x07, 0xF0]).unwrap();
    let d = Decodificador::nuevo(&m);
    let primera = d.decodificar(&mem, 0).unwrap();
    assert_eq!(primera.indice, indice("LDI"));
    assert_eq!(valor(&primera, "imm"), 7);
    let segunda = d.decodificar(&mem, 2).unwrap();
    assert_eq!(segunda.indice, indice("HLT"));
    assert_eq!(segunda.direccion, 2);
}

#[test]
fn decodificar_fuera_de_las_regiones_da_error_de_memoria() {
    let m = ejemplo_8();
    let mem = Memoria::nueva(&m).unwrap();
    assert!(matches!(
        Decodificador::nuevo(&m).decodificar(&mem, 0xF800),
        Err(ErrorEjecucion::FueraDeRegion { .. })
    ));
}

#[test]
fn si_la_segunda_palabra_cae_fuera_de_las_regiones_se_informa_de_esa_direccion() {
    let m = ejemplo_8();
    let mut mem = Memoria::nueva(&m).unwrap();
    mem.escribir_privilegiado(0xF0FF, 0x20).unwrap();
    assert!(matches!(
        Decodificador::nuevo(&m).decodificar(&mem, 0xF0FF),
        Err(ErrorEjecucion::FueraDeRegion { direccion: 0xF100 })
    ));
}
