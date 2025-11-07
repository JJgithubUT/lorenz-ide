//! Lecturas de registros que nadie ha escrito (RF-DEP-040).

mod comun;
use comun::ejemplo_8;
use lorenz_emu::estado::LecturaDeRegistro;
use lorenz_emu::Emulador;

// LDI A, 1 | LDI B, 2 | ADD A, B | LDI B, 0 | NOP | HLT (ejemplo-8)
const LDI_A_1: [u64; 2] = [0x20, 0x01];
const LDI_B_2: [u64; 2] = [0x24, 0x02];
const LDI_B_0: [u64; 2] = [0x24, 0x00];
const ADD_A_B: [u64; 1] = [0x31];
const NOP: [u64; 1] = [0x00];
const HLT: [u64; 1] = [0xF0];

fn con_programa(trozos: &[&[u64]]) -> Emulador {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    let palabras: Vec<u64> = trozos.iter().flat_map(|t| t.iter().copied()).collect();
    e.cargar_programa(0, &palabras).unwrap();
    e.estado_mut().activar_seguimiento();
    e
}

fn lecturas(e: &mut Emulador, pasos: usize) -> Vec<LecturaDeRegistro> {
    for _ in 0..pasos {
        e.paso().unwrap();
    }
    e.estado_mut().tomar_lecturas_sin_inicializar()
}

#[test]
fn leer_un_registro_no_escrito_se_anota_con_la_instruccion() {
    let mut e = con_programa(&[&LDI_A_1, &ADD_A_B, &HLT]);
    assert_eq!(
        lecturas(&mut e, 2),
        vec![LecturaDeRegistro {
            registro: "B".into(),
            pc: 2
        }]
    );
}

#[test]
fn escribir_antes_de_leer_no_avisa() {
    let mut e = con_programa(&[&LDI_A_1, &LDI_B_2, &ADD_A_B, &HLT]);
    assert!(lecturas(&mut e, 3).is_empty());
}

#[test]
fn escribir_el_mismo_valor_que_ya_tenia_tambien_inicializa() {
    // B ya valia 0; cargarle 0 lo inicializa igual, y no es lo mismo que no tocarlo.
    let mut e = con_programa(&[&LDI_B_0, &ADD_A_B, &HLT]);
    let l = lecturas(&mut e, 2);
    assert_eq!(
        l.iter().map(|x| x.registro.as_str()).collect::<Vec<_>>(),
        vec!["A"]
    );
}

#[test]
fn sin_seguimiento_no_se_anota_nada() {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.cargar_programa(0, &[0x31, 0xF0]).unwrap();
    e.paso().unwrap();
    assert!(e.estado_mut().tomar_lecturas_sin_inicializar().is_empty());
    assert!(!e.estado().seguimiento_activo());
}

#[test]
fn el_pc_y_el_sp_cuentan_como_inicializados() {
    let mut e = con_programa(&[&NOP, &HLT]);
    assert!(lecturas(&mut e, 2).is_empty());
}

#[test]
fn la_misma_lectura_en_la_misma_instruccion_no_se_repite() {
    // ADD A, B en un bucle: A y B se anotan una vez cada una.
    let mut e = con_programa(&[&ADD_A_B, &[0x60, 0x00, 0x00]]);
    let l = lecturas(&mut e, 6);
    assert_eq!(l.len(), 2, "{l:?}");
}

#[test]
fn reiniciar_vuelve_a_dejar_los_registros_sin_inicializar() {
    let mut e = con_programa(&[&LDI_B_2, &ADD_A_B, &HLT]);
    lecturas(&mut e, 2);
    e.reiniciar();
    let l = lecturas(&mut e, 0);
    assert!(l.is_empty());
    let l = lecturas(&mut e, 2);
    assert_eq!(l.len(), 1, "A se lee sin escribir tras reiniciar: {l:?}");
    assert_eq!(l[0].registro, "A");
}
