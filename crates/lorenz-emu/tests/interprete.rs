//! Pruebas del interprete de semantica: aritmetica, memoria, saltos y banderas.

mod comun;

use comun::Banco;
use lorenz_emu::ErrorEjecucion;

fn banco(semantica: &str) -> Banco {
    Banco::nuevo(semantica, "[]")
}

#[test]
fn una_semantica_vacia_no_hace_nada() {
    let mut b = banco("");
    b.fijar("A", 5);
    let r = b.ejecutar(0, 1).unwrap();
    assert_eq!(b.reg("A"), 5);
    assert!(!r.salto && !r.detenida);
}

#[test]
fn asignar_un_registro_desde_otro_por_operando() {
    let mut b = banco("rd = rs");
    b.fijar("B", 9);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(b.reg("A"), 9);
}

#[test]
fn la_suma_se_trunca_al_ancho_del_destino() {
    let mut b = banco("rd = rd + rs");
    b.fijar("A", 200);
    b.fijar("B", 100);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(b.reg("A"), 44);
}

#[test]
fn la_resta_por_debajo_de_cero_da_complemento() {
    let mut b = banco("rd = rd - rs");
    b.fijar("A", 1);
    b.fijar("B", 2);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(b.reg("A"), 0xFF);
}

#[test]
fn producto_division_y_modulo() {
    let mut b = banco("A = 7 * 6; B = A / 5; C = A % 5");
    b.ejecutar(0, 0).unwrap();
    assert_eq!((b.reg("A"), b.reg("B"), b.reg("C")), (42, 8, 2));
}

#[test]
fn dividir_entre_cero_da_error_con_el_mnemonico() {
    let mut b = banco("A = 1 / B");
    assert_eq!(b.ejecutar(0, 0), Err(ErrorEjecucion::DivisionEntreCero { mnemonico: "X".into() }));
}

#[test]
fn modulo_entre_cero_tambien_da_error() {
    let mut b = banco("A = 1 % B");
    assert!(matches!(b.ejecutar(0, 0), Err(ErrorEjecucion::DivisionEntreCero { .. })));
}

#[test]
fn operaciones_de_bits() {
    let mut b = banco("A = 0b1100 & 0b1010; B = 0b1100 | 0b1010; C = 0b1100 ^ 0b1010; D = ~0");
    b.ejecutar(0, 0).unwrap();
    assert_eq!((b.reg("A"), b.reg("B"), b.reg("C"), b.reg("D")), (0b1000, 0b1110, 0b0110, 0xFF));
}

#[test]
fn desplazamientos() {
    let mut b = banco("A = 1 << 7; B = 0x80 >> 3; C = 1 << 9");
    b.ejecutar(0, 0).unwrap();
    assert_eq!((b.reg("A"), b.reg("B"), b.reg("C")), (0x80, 0x10, 0));
}

#[test]
fn un_desplazamiento_enorme_no_desborda() {
    let mut b = banco("A = 1 << 200; B = 5 >> 200");
    b.ejecutar(0, 0).unwrap();
    assert_eq!((b.reg("A"), b.reg("B")), (0, 0));
}

#[test]
fn comparaciones_dan_uno_o_cero() {
    let mut b = banco("A = 3 < 4; B = 3 >= 4; C = 3 == 3; D = 3 != 3");
    b.ejecutar(0, 0).unwrap();
    assert_eq!((b.reg("A"), b.reg("B"), b.reg("C"), b.reg("D")), (1, 0, 1, 0));
}

#[test]
fn negacion_logica_y_menos_unario() {
    let mut b = banco("A = !0; B = !5; C = -1");
    b.ejecutar(0, 0).unwrap();
    assert_eq!((b.reg("A"), b.reg("B"), b.reg("C")), (1, 0, 0xFF));
}

#[test]
fn la_precedencia_sigue_a_rust() {
    let mut b = banco("A = 2 + 3 * 4; B = 1 == 1 | 0");
    b.ejecutar(0, 0).unwrap();
    assert_eq!((b.reg("A"), b.reg("B")), (14, 1));
}

#[test]
fn el_y_logico_no_evalua_el_lado_derecho_si_el_izquierdo_es_falso() {
    let mut b = banco("A = 0 && (1 / 0)");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.reg("A"), 0);
}

#[test]
fn el_o_logico_no_evalua_el_lado_derecho_si_el_izquierdo_es_cierto() {
    let mut b = banco("A = 1 || (1 / 0)");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.reg("A"), 1);
}

#[test]
fn escribir_y_leer_memoria() {
    let mut b = banco("mem[0x10] = 77; A = mem[0x10]");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.reg("A"), 77);
    assert_eq!(b.memoria.leer(0x10).unwrap(), 77);
}

#[test]
fn la_direccion_de_memoria_puede_ser_una_expresion() {
    let mut b = banco("mem[rd + 1] = 5");
    b.fijar("A", 0x20);
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.memoria.leer(0x21).unwrap(), 5);
}

#[test]
fn escribir_memoria_trunca_a_una_palabra() {
    let mut b = banco("mem[0] = 0x1FF");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.memoria.leer(0).unwrap(), 0xFF);
}

#[test]
fn escribir_en_solo_lectura_da_error() {
    let mut b = banco("mem[0x100] = 1");
    assert!(matches!(b.ejecutar(0, 0), Err(ErrorEjecucion::EscrituraEnSoloLectura { .. })));
}

#[test]
fn leer_de_solo_escritura_da_error() {
    let mut b = banco("A = mem[0x200]");
    assert!(matches!(b.ejecutar(0, 0), Err(ErrorEjecucion::LecturaDeSoloEscritura { .. })));
}

#[test]
fn leer_fuera_de_las_regiones_da_error() {
    let mut b = banco("A = mem[0x800]");
    assert!(matches!(b.ejecutar(0, 0), Err(ErrorEjecucion::FueraDeRegion { direccion: 0x800 })));
}

#[test]
fn la_direccion_se_trunca_a_los_bits_de_direccion() {
    let mut b = banco("mem[0x10000 + 5] = 3");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.memoria.leer(5).unwrap(), 3);
}

#[test]
fn leer_pc_da_la_direccion_de_la_instruccion_siguiente() {
    let mut b = banco("A = pc");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.reg("A"), 1);
}

#[test]
fn asignar_pc_es_un_salto() {
    let mut b = banco("pc = 0x40");
    let r = b.ejecutar(0, 0).unwrap();
    assert!(r.salto);
    assert_eq!(b.estado.pc(), 0x40);
}

#[test]
fn asignar_pc_por_su_nombre_declarado_tambien_es_un_salto() {
    let mut b = banco("PC = 0x41");
    assert!(b.ejecutar(0, 0).unwrap().salto);
    assert_eq!(b.estado.pc(), 0x41);
}

#[test]
fn sin_asignar_pc_no_hay_salto() {
    let mut b = banco("A = 1");
    assert!(!b.ejecutar(0, 0).unwrap().salto);
}

#[test]
fn si_ejecuta_el_cuerpo_cuando_la_condicion_es_cierta() {
    let mut b = banco("si A == 0: B = 5");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.reg("B"), 5);
}

#[test]
fn si_omite_el_cuerpo_cuando_la_condicion_es_falsa() {
    let mut b = banco("si A != 0: B = 5");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.reg("B"), 0);
}

#[test]
fn si_con_llaves_ejecuta_todo_el_cuerpo() {
    let mut b = banco("si 1: { A = 1; B = 2 }");
    b.ejecutar(0, 0).unwrap();
    assert_eq!((b.reg("A"), b.reg("B")), (1, 2));
}

#[test]
fn salto_condicional_por_bandera() {
    let mut b = banco("si Z: pc = 0x30");
    assert!(!b.ejecutar(0, 0).unwrap().salto);
    b.estado.fijar_bandera("Z", true).unwrap();
    assert!(b.ejecutar(0, 0).unwrap().salto);
    assert_eq!(b.estado.pc(), 0x30);
}

#[test]
fn detener_marca_la_maquina_y_corta_la_ejecucion() {
    let mut b = banco("A = 1; detener; B = 2");
    let r = b.ejecutar(0, 0).unwrap();
    assert!(r.detenida);
    assert!(b.estado.detenida());
    assert_eq!((b.reg("A"), b.reg("B")), (1, 0));
}

#[test]
fn detener_dentro_de_un_si_corta_lo_que_sigue() {
    let mut b = banco("si 1: detener; A = 9");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.reg("A"), 0);
}

#[test]
fn apilar_con_sp() {
    let mut b = banco("sp = sp - 1; mem[sp] = rs");
    b.fijar("B", 0x5A);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(b.estado.sp(), Some(0xFE));
    assert_eq!(b.memoria.leer(0xFE).unwrap(), 0x5A);
}

#[test]
fn entrada_y_salida_por_puertos() {
    let mut b = banco("A = in[1]; out[2] = A + 1");
    b.puertos.encolar_entrada(1, 41).unwrap();
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.reg("A"), 41);
    assert_eq!(b.puertos.salida(2).unwrap(), [42]);
}

#[test]
fn un_puerto_no_declarado_da_error_al_leer() {
    let mut b = banco("A = in[9]");
    assert!(matches!(b.ejecutar(0, 0), Err(ErrorEjecucion::PuertoNoDeclarado { puerto: 9 })));
}

#[test]
fn un_puerto_no_declarado_da_error_al_escribir() {
    let mut b = banco("out[9] = 1");
    assert!(matches!(b.ejecutar(0, 0), Err(ErrorEjecucion::PuertoNoDeclarado { puerto: 9 })));
}

#[test]
fn la_salida_se_trunca_a_una_palabra() {
    let mut b = banco("out[1] = 0x1AB");
    b.ejecutar(0, 0).unwrap();
    assert_eq!(b.puertos.salida(1).unwrap(), [0xAB]);
}

#[test]
fn leer_una_bandera_da_uno_o_cero() {
    let mut b = banco("A = Z; B = C");
    b.estado.fijar_bandera("Z", true).unwrap();
    b.ejecutar(0, 0).unwrap();
    assert_eq!((b.reg("A"), b.reg("B")), (1, 0));
}

#[test]
fn asignar_una_bandera_de_forma_explicita() {
    let mut b = banco("V = 1");
    b.ejecutar(0, 0).unwrap();
    assert!(b.estado.bandera("V").unwrap());
}

#[test]
fn asignar_cero_apaga_una_bandera() {
    let mut b = banco("Z = 0");
    b.estado.fijar_bandera("Z", true).unwrap();
    b.ejecutar(0, 0).unwrap();
    assert!(!b.estado.bandera("Z").unwrap());
}
