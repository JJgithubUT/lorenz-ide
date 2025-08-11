//! Pruebas de la convencion de banderas Z, N y C (D-013).

mod comun;

use comun::Banco;

fn suma(afecta: &str) -> Banco {
    Banco::nuevo("rd = rd + rs", afecta)
}

fn flags(b: &Banco) -> (bool, bool, bool) {
    (
        b.estado.bandera("Z").unwrap(),
        b.estado.bandera("N").unwrap(),
        b.estado.bandera("C").unwrap(),
    )
}

#[test]
fn la_suma_con_resultado_cero_enciende_z() {
    let mut b = suma(r#"["Z", "N", "C"]"#);
    b.fijar("A", 0);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(flags(&b), (true, false, false));
}

#[test]
fn la_suma_con_bit_alto_enciende_n() {
    let mut b = suma(r#"["Z", "N", "C"]"#);
    b.fijar("A", 0x70);
    b.fijar("B", 0x10);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(flags(&b), (false, true, false));
}

#[test]
fn la_suma_que_se_pasa_del_ancho_enciende_c() {
    let mut b = suma(r#"["Z", "N", "C"]"#);
    b.fijar("A", 0xFF);
    b.fijar("B", 0x02);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(b.reg("A"), 1);
    assert_eq!(flags(&b), (false, false, true));
}

#[test]
fn la_suma_que_da_cero_con_acarreo_enciende_z_y_c() {
    let mut b = suma(r#"["Z", "N", "C"]"#);
    b.fijar("A", 0x80);
    b.fijar("B", 0x80);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(flags(&b), (true, false, true));
}

#[test]
fn una_suma_sin_acarreo_apaga_c() {
    let mut b = suma(r#"["C"]"#);
    b.estado.fijar_bandera("C", true).unwrap();
    b.fijar("A", 1);
    b.ejecutar(0, 1).unwrap();
    assert!(!b.estado.bandera("C").unwrap());
}

#[test]
fn la_resta_con_prestamo_enciende_c() {
    let mut b = Banco::nuevo("rd = rd - rs", r#"["Z", "N", "C"]"#);
    b.fijar("A", 1);
    b.fijar("B", 2);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(b.reg("A"), 0xFF);
    assert_eq!(flags(&b), (false, true, true));
}

#[test]
fn la_resta_sin_prestamo_no_enciende_c() {
    let mut b = Banco::nuevo("rd = rd - rs", r#"["Z", "C"]"#);
    b.fijar("A", 5);
    b.fijar("B", 5);
    b.ejecutar(0, 1).unwrap();
    assert!(b.estado.bandera("Z").unwrap());
    assert!(!b.estado.bandera("C").unwrap());
}

#[test]
fn el_producto_que_se_pasa_enciende_c() {
    let mut b = Banco::nuevo("rd = rd * rs", r#"["C"]"#);
    b.fijar("A", 16);
    b.fijar("B", 16);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(b.reg("A"), 0);
    assert!(b.estado.bandera("C").unwrap());
}

#[test]
fn el_desplazamiento_a_la_izquierda_que_pierde_bits_enciende_c() {
    let mut b = Banco::nuevo("rd = rd << 1", r#"["C"]"#);
    b.fijar("A", 0x80);
    b.ejecutar(0, 0).unwrap();
    assert!(b.estado.bandera("C").unwrap());
}

#[test]
fn una_operacion_de_bits_nunca_enciende_c() {
    let mut b = Banco::nuevo("rd = rd | rs", r#"["Z", "C"]"#);
    b.estado.fijar_bandera("C", true).unwrap();
    b.fijar("A", 0xFF);
    b.fijar("B", 0xFF);
    b.ejecutar(0, 1).unwrap();
    assert!(!b.estado.bandera("C").unwrap());
}

#[test]
fn las_banderas_que_no_se_listan_quedan_intactas() {
    let mut b = suma(r#"["Z"]"#);
    b.estado.fijar_bandera("C", true).unwrap();
    b.estado.fijar_bandera("N", true).unwrap();
    b.fijar("A", 1);
    b.ejecutar(0, 1).unwrap();
    assert_eq!(flags(&b), (false, true, true));
}

#[test]
fn una_instruccion_sin_afecta_no_toca_ninguna_bandera() {
    let mut b = suma("[]");
    b.estado.fijar_bandera("Z", true).unwrap();
    b.fijar("A", 1);
    b.ejecutar(0, 1).unwrap();
    assert!(b.estado.bandera("Z").unwrap());
}

#[test]
fn si_no_se_asigna_nada_las_banderas_quedan_como_estaban() {
    let mut b = Banco::nuevo("si 0: A = 1", r#"["Z", "N", "C"]"#);
    b.estado.fijar_bandera("Z", true).unwrap();
    b.ejecutar(0, 0).unwrap();
    assert_eq!(flags(&b), (true, false, false));
}

#[test]
fn una_bandera_explicita_manda_sobre_la_convencion() {
    let mut b = Banco::nuevo("rd = rd + rs; Z = 1", r#"["Z", "C"]"#);
    b.fijar("A", 1);
    b.ejecutar(0, 1).unwrap();
    assert!(b.estado.bandera("Z").unwrap());
}

#[test]
fn una_bandera_no_convencional_solo_cambia_si_se_asigna() {
    let mut b = Banco::nuevo("rd = rd + rs", r#"["V"]"#);
    b.estado.fijar_bandera("V", true).unwrap();
    b.fijar("A", 1);
    b.ejecutar(0, 1).unwrap();
    assert!(b.estado.bandera("V").unwrap());
}

#[test]
fn las_banderas_se_calculan_con_el_ultimo_valor_asignado() {
    let mut b = Banco::nuevo("A = 0; B = 5", r#"["Z"]"#);
    b.ejecutar(0, 0).unwrap();
    assert!(!b.estado.bandera("Z").unwrap());
}

#[test]
fn la_bandera_n_usa_el_ancho_del_destino() {
    // Un registro de 16 bits (SP) con 0x0080 no es negativo aunque su byte bajo valga 0x80.
    let mut b = Banco::nuevo("sp = 0x80", r#"["N"]"#);
    b.ejecutar(0, 0).unwrap();
    assert!(!b.estado.bandera("N").unwrap());
}

#[test]
fn escribir_en_memoria_tambien_marca_z() {
    let mut b = Banco::nuevo("mem[0] = 0", r#"["Z"]"#);
    b.ejecutar(0, 0).unwrap();
    assert!(b.estado.bandera("Z").unwrap());
}
