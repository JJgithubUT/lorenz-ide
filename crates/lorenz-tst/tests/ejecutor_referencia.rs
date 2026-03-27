//! La referencia como segunda maquina completa (RF-TST-025).

use std::path::PathBuf;

use lorenz_arch::modelo::Maquina;
use lorenz_emu::{comparar, Emulador, Veredicto};
use lorenz_ens::ensamblador::{Ensamblador, Fuente};
use lorenz_tst::ejecutor_referencia::EjecutorReferencia;

fn maquina(ruta: &str) -> Maquina {
    lorenz_arch::carga::cargar_archivo(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ruta))
        .unwrap()
}

/// Corre el mismo programa en el emulador y en la referencia y compara paso a paso.
fn comparar_programa(m: &Maquina, fuente: &str, limite: u64) -> (Veredicto, bool) {
    let objeto = Ensamblador::nuevo(m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", fuente))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut emu = Emulador::nuevo(m.clone()).unwrap();
    let mut referencia = EjecutorReferencia::nuevo(m).unwrap();
    for s in &objeto.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
        referencia.cargar(s.direccion, &s.palabras).unwrap();
    }
    let v = comparar(&mut emu, &mut referencia, limite);
    (v, referencia.no_modelado)
}

#[test]
fn el_programa_de_suma_da_lo_mismo_en_las_dos() {
    let m = maquina("../../ejemplos/ejemplo-8/maquina.toml");
    let (v, _) = comparar_programa(
        &m,
        "LDI A, 200\nLDI B, 100\nADD A, B\nST [0x2000], A\nLD C, [0x2000]\nHLT\n",
        100,
    );
    assert!(
        matches!(v, Veredicto::Iguales { instrucciones: 6 }),
        "{v:?}"
    );
}

#[test]
fn un_bucle_con_saltos_condicionales_tambien() {
    let m = maquina("../../ejemplos/ejemplo-8/maquina.toml");
    let (v, _) = comparar_programa(
        &m,
        "LDI A, 250\nLDI B, 1\nbucle: ADD A, B\nJZ fin\nLDI C, 0\nJZ bucle\nfin: HLT\n",
        1000,
    );
    assert!(matches!(v, Veredicto::Iguales { .. }), "{v:?}");
}

#[test]
fn la_pila_las_llamadas_y_los_saltos_relativos_coinciden() {
    let m = maquina("tests/maquinas/rica-8.toml");
    let (v, _) = comparar_programa(
        &m,
        "
        LDI A, 5
        PUSH A
        CALL sub
        POP B
        HLT
sub:    LDI C, 3
        ADD A, C
        SHL A
        NEG A
        INC A
        RET
",
        1000,
    );
    assert!(matches!(v, Veredicto::Iguales { .. }), "{v:?}");
}

#[test]
fn la_resta_y_la_division_entre_cero_fallan_igual() {
    let m = maquina("tests/maquinas/rica-8.toml");
    let (v, _) = comparar_programa(&m, "LDI A, 9\nLDI B, 0\nSUB A, B\nDIV A, B\nHLT\n", 100);
    assert!(
        matches!(v, Veredicto::Iguales { instrucciones: 3 }),
        "{v:?}"
    );
}

#[test]
fn escribir_en_rom_falla_igual_y_deja_el_estado_igual() {
    let m = maquina("../../ejemplos/ejemplo-8/maquina.toml");
    let (v, _) = comparar_programa(&m, "LDI A, 7\nST [0x0100], A\nHLT\n", 100);
    assert!(matches!(v, Veredicto::Iguales { .. }), "{v:?}");
}

#[test]
fn llegar_a_un_dispositivo_se_marca_como_no_modelado() {
    let m = maquina("../../ejemplos/ejemplo-8/maquina.toml");
    let (_, no_modelado) = comparar_programa(&m, "LDI A, 7\nST [0xF000], A\nHLT\n", 100);
    assert!(no_modelado);
}
