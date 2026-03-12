//! Inyeccion de fallos en instantes elegidos (RF-DEP-042).

use lorenz_dbg::inyeccion::Inyeccion;
use lorenz_dbg::{Depurador, ErrorDepurador};
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

const PROGRAMA: &str = "\
        LDI A, 1
        LDI B, 2
        ADD A, B
        ST [0x2000], A
        HLT
";

fn depurador() -> Depurador {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", PROGRAMA))
        .unwrap();
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    Depurador::nuevo(emu)
}

fn bit_a(bit: u32) -> Inyeccion {
    Inyeccion::BitRegistro {
        nombre: "A".into(),
        bit,
    }
}

#[test]
fn sin_fallos_el_resultado_es_el_esperado() {
    let mut d = depurador();
    d.continuar(100);
    assert_eq!(d.emulador().memoria().leer_privilegiado(0x2000).unwrap(), 3);
}

#[test]
fn un_bit_alterado_cambia_el_resultado_final() {
    let mut d = depurador();
    // Tras LDI A,1 y LDI B,2 (dos instrucciones), A pasa de 1 a 5.
    d.programar_inyeccion(2, bit_a(2)).unwrap();
    d.continuar(100);
    assert_eq!(d.emulador().memoria().leer_privilegiado(0x2000).unwrap(), 7);
}

#[test]
fn el_fallo_actua_justo_en_su_instante_y_no_antes() {
    let mut d = depurador();
    d.programar_inyeccion(2, bit_a(2)).unwrap();
    d.paso(1);
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 1);
    d.paso(2); // la inyeccion actua antes de la tercera instruccion
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 7);
    assert!(d.inyecciones()[0].aplicada);
}

#[test]
fn cada_fallo_se_aplica_una_sola_vez() {
    let mut d = depurador();
    d.programar_inyeccion(0, bit_a(3)).unwrap();
    d.continuar(100);
    assert_eq!(d.emulador().memoria().leer_privilegiado(0x2000).unwrap(), 3);
    assert_eq!(
        d.tomar_avisos()
            .iter()
            .filter(|a| a.contains("inyectado"))
            .count(),
        1
    );
}

#[test]
fn un_bit_de_memoria_se_invierte_en_su_instante() {
    let mut d = depurador();
    d.programar_inyeccion(
        4,
        Inyeccion::BitMemoria {
            direccion: 0x2000,
            bit: 0,
        },
    )
    .unwrap();
    d.continuar(100);
    assert_eq!(d.emulador().memoria().leer_privilegiado(0x2000).unwrap(), 2);
}

#[test]
fn el_aviso_dice_que_se_inyecto_y_donde() {
    let mut d = depurador();
    let id = d.programar_inyeccion(1, bit_a(0)).unwrap();
    d.continuar(100);
    let avisos = d.tomar_avisos();
    assert!(
        avisos
            .iter()
            .any(|a| a == &format!("fallo {id} inyectado en el instante 1: bit 0 de A")),
        "{avisos:?}"
    );
}

#[test]
fn un_bit_fuera_del_ancho_se_rechaza() {
    let mut d = depurador();
    assert_eq!(
        d.programar_inyeccion(0, bit_a(8)),
        Err(ErrorDepurador::BitFueraDeRango { bit: 8, ancho: 8 })
    );
    assert!(matches!(
        d.programar_inyeccion(
            0,
            Inyeccion::BitRegistro {
                nombre: "ZZ".into(),
                bit: 0
            }
        ),
        Err(ErrorDepurador::NombreDesconocido(_))
    ));
}

#[test]
fn un_fallo_quitado_no_actua() {
    let mut d = depurador();
    let id = d.programar_inyeccion(2, bit_a(2)).unwrap();
    d.quitar_inyeccion(id).unwrap();
    d.continuar(100);
    assert_eq!(d.emulador().memoria().leer_privilegiado(0x2000).unwrap(), 3);
    assert!(d.quitar_inyeccion(id).is_err());
}

#[test]
fn un_fallo_que_no_se_puede_aplicar_queda_como_aviso_y_no_detiene() {
    let mut d = depurador();
    d.programar_inyeccion(1, Inyeccion::Interrupcion { linea: 9 })
        .unwrap();
    d.continuar(100);
    assert!(d
        .tomar_avisos()
        .iter()
        .any(|a| a.contains("no se pudo inyectar")));
    assert_eq!(d.emulador().memoria().leer_privilegiado(0x2000).unwrap(), 3);
}

#[test]
fn con_traza_se_puede_retroceder_y_volver_a_ver_el_fallo() {
    let mut d = depurador();
    d.grabar(true);
    d.programar_inyeccion(2, bit_a(2)).unwrap();
    d.continuar(100);
    let final_ = d.emulador().memoria().leer_privilegiado(0x2000).unwrap();
    d.ir_a(1).unwrap();
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 1);
    d.ir_a(3).unwrap();
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 7);
    d.adelante(10).unwrap();
    assert_eq!(
        d.emulador().memoria().leer_privilegiado(0x2000).unwrap(),
        final_
    );
}
