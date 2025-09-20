//! Desensamblado y ida y vuelta con el ensamblador (RF-ENS-021 a RF-ENS-025).

mod comun;
use std::collections::BTreeMap;

use comun::*;
use lorenz_ens::desensamblador::{Desensamblador, Linea};

fn textos(m: &lorenz_arch::modelo::Maquina, palabras: &[u64]) -> Vec<String> {
    Desensamblador::nuevo(m)
        .unwrap()
        .desensamblar(0, palabras, None)
        .into_iter()
        .map(|l| match l {
            Linea::Instruccion { texto, .. } => texto,
            Linea::Dato { palabra, .. } => format!("dato {palabra:#04x}"),
        })
        .collect()
}

#[test]
fn reconstruye_el_texto_de_cada_instruccion() {
    let m = ejemplo_8();
    let t = textos(&m, &[0x20, 0xC8, 0x24, 0x64, 0x31, 0xF0]);
    assert_eq!(t, vec!["LDI A, 200", "LDI B, 100", "ADD A, B", "HLT"]);
}

#[test]
fn las_direcciones_se_escriben_en_hexadecimal_del_ancho_del_espacio() {
    let m = ejemplo_8();
    assert_eq!(textos(&m, &[0x48, 0x10, 0x20]), vec!["LD C, [0x2010]"]);
    assert_eq!(textos(&m, &[0x5C, 0x00, 0x20]), vec!["ST [0x2000], D"]);
}

#[test]
fn una_palabra_sin_codificacion_se_marca_como_dato() {
    let m = ejemplo_8();
    // 0x90 no es ninguna instruccion de ejemplo-8.
    let t = textos(&m, &[0x90, 0xF0]);
    assert_eq!(t, vec!["dato 0x90", "HLT"]);
}

#[test]
fn una_instruccion_truncada_por_el_final_se_marca_como_dato() {
    let m = ejemplo_8();
    // LDI necesita dos palabras y solo hay una.
    assert_eq!(textos(&m, &[0x20]), vec!["dato 0x20"]);
}

#[test]
fn los_bits_de_relleno_no_cambian_la_instruccion() {
    let m = ejemplo_8();
    // NOP es 0000 .... : cualquier valor de los 4 bajos es un NOP.
    assert_eq!(textos(&m, &[0x07]), vec!["NOP"]);
}

#[test]
fn las_direcciones_y_longitudes_de_las_lineas_son_correctas() {
    let m = ejemplo_8();
    let d = Desensamblador::nuevo(&m).unwrap();
    let l = d.desensamblar(0x100, &[0x20, 0x01, 0xF0], None);
    assert_eq!(l[0].direccion(), 0x100);
    assert_eq!(l[0].longitud(), 2);
    assert_eq!(l[1].direccion(), 0x102);
}

#[test]
fn con_simbolos_las_direcciones_se_vuelven_etiquetas() {
    let m = ejemplo_8();
    let d = Desensamblador::nuevo(&m).unwrap();
    let mut s = BTreeMap::new();
    s.insert(3u64, "fin".to_string());
    let l = d.desensamblar(0, &[0x60, 0x03, 0x00, 0xF0], Some(&s));
    match &l[0] {
        Linea::Instruccion { texto, .. } => assert_eq!(texto, "JZ fin"),
        otra => panic!("{otra:?}"),
    }
    let texto = d.a_texto(&l, Some(&s));
    assert!(texto.contains("fin:\n"));
}

#[test]
fn el_salto_relativo_se_escribe_como_direccion_de_destino() {
    let m = con_saltos_relativos("little");
    // JR con desplazamiento -2 en la direccion 0 salta a si mismo.
    assert_eq!(textos(&m, &[0x70, 0xFE]), vec!["JR 0x0000"]);
}

#[test]
fn ida_y_vuelta_de_un_programa_con_etiquetas() {
    let m = ejemplo_8();
    let fuente = "inicio: LDI A, 3\nbucle: LDI B, 1\nADD A, B\nJZ inicio\nST [0x2000], A\nHLT\n";
    ida_y_vuelta(&m, fuente);
}

#[test]
fn ida_y_vuelta_con_datos_y_varios_segmentos() {
    let m = ejemplo_8();
    ida_y_vuelta(
        &m,
        "HLT\n.org 0x0100\n.db 0x91, 0x92, 0x90\nNOP\n.org 0x2000\nMOV A, B\n",
    );
}

#[test]
fn ida_y_vuelta_en_big_endian_con_saltos_relativos() {
    let m = con_saltos_relativos("big");
    ida_y_vuelta(
        &m,
        "a: JR a\nJP 0x1234\nOUTP 7\nADDI -5\nJR b\nNOP\nb: NOP\n",
    );
}

#[test]
fn ida_y_vuelta_de_todas_las_instrucciones_con_todos_los_registros() {
    let m = ejemplo_8();
    let mut fuente = String::new();
    for rd in ["A", "B", "C", "D"] {
        for rs in ["A", "B", "C", "D"] {
            fuente.push_str(&format!("MOV {rd}, {rs}\nADD {rd}, {rs}\n"));
        }
        fuente.push_str(&format!(
            "LDI {rd}, 255\nLD {rd}, [0x2FFF]\nST [0x2001], {rd}\n"
        ));
    }
    fuente.push_str("NOP\nJZ 0x0000\nRETI\nEI\nDI\nHLT\n");
    ida_y_vuelta(&m, &fuente);
}

fn ida_y_vuelta(m: &lorenz_arch::modelo::Maquina, fuente: &str) {
    let o1 = ensamblar(m, fuente).unwrap_or_else(|e| panic!("{e:?}"));
    let d = Desensamblador::nuevo(m).unwrap();
    let mut texto = String::new();
    for s in &o1.segmentos {
        let lineas = d.desensamblar(s.direccion, &s.palabras, None);
        texto.push_str(&d.a_texto(&lineas, None));
    }
    let o2 = ensamblar(m, &texto).unwrap_or_else(|e| panic!("{e:?}\n{texto}"));
    assert_eq!(o1.palabras(), o2.palabras(), "texto generado:\n{texto}");
}
