//! Ensamblado de programas sencillos (RF-ENS-001, RF-ENS-002, RF-ENS-026).

mod comun;
use comun::*;

#[test]
fn el_programa_suma_coincide_con_su_version_en_hexadecimal() {
    let m = ejemplo_8();
    let fuente = "LDI A, 200\nLDI B, 100\nADD A, B\nHLT\n";
    assert_eq!(bytes(&m, fuente), vec![0x20, 0xC8, 0x24, 0x64, 0x31, 0xF0]);
}

#[test]
fn los_mnemonicos_y_registros_no_distinguen_mayusculas() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, "ldi a, 1\nhlt"), bytes(&m, "LDI A, 1\nHLT"));
}

#[test]
fn el_espaciado_es_libre() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, "   LDI   A  ,   5   "), vec![0x20, 5]);
    assert_eq!(bytes(&m, "MOV B,C"), vec![0x16]);
}

#[test]
fn los_comentarios_y_lineas_vacias_se_ignoran() {
    let m = ejemplo_8();
    let t = "; encabezado\n\nNOP ; sin efecto\n\n  ; fin\nHLT\n";
    assert_eq!(bytes(&m, t), vec![0x00, 0xF0]);
}

#[test]
fn un_programa_vacio_no_emite_nada() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "").unwrap();
    assert_eq!(o.tamano(), 0);
    assert!(o.segmentos.is_empty());
}

#[test]
fn el_programa_empieza_en_la_direccion_de_arranque() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "HLT").unwrap();
    assert_eq!(o.segmentos[0].direccion, m.arranque);
}

#[test]
fn cada_forma_de_una_instruccion_usa_su_codificacion() {
    let m = ejemplo_8();
    assert_eq!(bytes(&m, "LD C, [0x2010]"), vec![0x48, 0x10, 0x20]);
    assert_eq!(bytes(&m, "ST [0x2000], D"), vec![0x5C, 0x00, 0x20]);
    assert_eq!(bytes(&m, "JZ 0x0100"), vec![0x60, 0x00, 0x01]);
}

#[test]
fn el_ensamblador_lista_los_mnemonicos_de_la_maquina() {
    let m = ejemplo_8();
    let e = lorenz_ens::ensamblador::Ensamblador::nuevo(&m).unwrap();
    let mn = e.mnemonicos();
    assert!(mn.contains(&"LDI") && mn.contains(&"HLT") && mn.contains(&"RETI"));
    assert_eq!(mn.len(), m.instrucciones.len());
}

#[test]
fn una_instruccion_nueva_en_la_descripcion_aparece_sin_tocar_el_ensamblador() {
    let mut texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    texto.push_str(
        r#"
[[instruccion]]
mnemonico = "DOBLA"
sintaxis  = "DOBLA {rd}"
codigo    = "1110 rd:2 .."
semantica = "rd = rd + rd"
ciclos    = 1
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "B", "C", "D"]
"#,
    );
    let m = lorenz_arch::cargar(&texto).unwrap();
    assert_eq!(bytes(&m, "DOBLA C"), vec![0xE8]);
}
