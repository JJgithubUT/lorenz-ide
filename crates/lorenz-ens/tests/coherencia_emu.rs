//! El desensamblador y el decodificador del emulador deben leer las mismas palabras igual.
//!
//! Son dos implementaciones de la misma regla (D-029); esta prueba es la que las mantiene de acuerdo.

mod comun;
use comun::*;
use lorenz_emu::Decodificador;
use lorenz_ens::desensamblador::{Desensamblador, Linea};

fn comparar(m: &lorenz_arch::modelo::Maquina) {
    let dis = Desensamblador::nuevo(m).unwrap();
    let dec = Decodificador::nuevo(m);
    let mut comprobadas = 0;
    for a in 0..=m.palabra_maxima() {
        for b in [0u64, 0x55, 0xFF] {
            for c in [0u64, 0xAA] {
                let palabras = [a, b, c];
                let lineas = dis.desensamblar(0, &palabras, None);
                match dec.decodificar_palabras(0, &palabras) {
                    Ok(d) => match &lineas[0] {
                        Linea::Instruccion { indice, palabras, .. } => {
                            assert_eq!(*indice, d.indice, "palabra {a:#x}");
                            assert_eq!(palabras.len(), d.longitud, "palabra {a:#x}");
                        }
                        otra => panic!("{a:#x},{b:#x},{c:#x}: el emulador decodifica y el desensamblador dice {otra:?}"),
                    },
                    Err(_) => assert!(
                        matches!(lineas[0], Linea::Dato { .. }),
                        "{a:#x},{b:#x},{c:#x}: el emulador rechaza y el desensamblador decodifica {:?}",
                        lineas[0]
                    ),
                }
                comprobadas += 1;
            }
        }
    }
    assert!(comprobadas > 100);
}

#[test]
fn coinciden_en_ejemplo_8() {
    comparar(&ejemplo_8());
}

#[test]
fn coinciden_con_saltos_relativos_little_y_big() {
    comparar(&con_saltos_relativos("little"));
    comparar(&con_saltos_relativos("big"));
}

#[test]
fn lo_que_ensambla_lo_decodifica_el_emulador_con_los_mismos_operandos() {
    let m = ejemplo_8();
    let o = ensamblar(&m, "LD C, [0x2010]").unwrap();
    let palabras: Vec<u64> = o.palabras().into_iter().map(|(_, p)| p).collect();
    let d = decodificar(&m, &palabras);
    assert_eq!(d.0, "LD");
    assert_eq!(
        d.1,
        vec![("rd".to_string(), 2), ("dir".to_string(), 0x2010)]
    );
}

fn decodificar(m: &lorenz_arch::modelo::Maquina, p: &[u64]) -> (String, Vec<(String, i128)>) {
    let d = Decodificador::nuevo(m).decodificar_palabras(0, p).unwrap();
    (
        m.instrucciones[d.indice].mnemonico.clone(),
        d.operandos
            .into_iter()
            .map(|o| (o.nombre, o.valor))
            .collect(),
    )
}
