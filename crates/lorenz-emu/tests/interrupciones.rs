//! Pruebas de las interrupciones: vectores, pila, prioridad y habilitacion.

mod comun;

use comun::ejemplo_8;
use lorenz_arch::modelo::{Endianness, RolRegistro, SentidoPila};
use lorenz_emu::{Emulador, ErrorEjecucion};

/// Ejemplo de 8 bits con dos manejadores en la rom y los vectores 0 y 1 apuntando a ellos.
///
/// - 0x0000: NOP, NOP, HLT (programa principal)
/// - 0x0100: LDI C,0x99 ; HLT (manejador de la linea 0)
/// - 0x0110: LDI D,0x77 ; HLT (manejador de la linea 1)
fn emulador() -> Emulador {
    emulador_de(ejemplo_8())
}

fn emulador_de(m: lorenz_arch::modelo::Maquina) -> Emulador {
    let mut e = Emulador::nuevo(m).unwrap();
    e.cargar_programa(0x0000, &[0x00, 0x00, 0xF0]).unwrap();
    e.cargar_programa(0x0100, &[0x28, 0x99, 0xF0]).unwrap();
    e.cargar_programa(0x0110, &[0x2C, 0x77, 0xF0]).unwrap();
    e.cargar_programa(0xEFF0, &[0x00, 0x01]).unwrap();
    e.cargar_programa(0xEFF2, &[0x10, 0x01]).unwrap();
    e
}

#[test]
fn levantar_una_linea_sin_vector_da_error() {
    let mut e = emulador();
    assert_eq!(e.levantar_interrupcion(7), Err(ErrorEjecucion::InterrupcionNoDeclarada { linea: 7 }));
    assert!(e.interrupciones_pendientes().is_empty());
}

#[test]
fn una_maquina_sin_interrupciones_no_acepta_ninguna_linea() {
    let mut m = ejemplo_8();
    m.interrupciones = None;
    let mut e = Emulador::nuevo(m).unwrap();
    assert!(matches!(e.levantar_interrupcion(0), Err(ErrorEjecucion::InterrupcionNoDeclarada { .. })));
}

#[test]
fn con_las_interrupciones_inhabilitadas_la_linea_queda_pendiente() {
    let mut e = emulador();
    e.levantar_interrupcion(0).unwrap();
    e.paso().unwrap();
    assert_eq!(e.estado().pc(), 1);
    assert_eq!(e.interrupciones_pendientes(), [0]);
}

#[test]
fn habilitadas_se_salta_al_manejador_del_vector() {
    let mut e = emulador();
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    let p = e.paso().unwrap().unwrap();
    // Se atendio antes de ejecutar nada: la primera instruccion es la del manejador.
    assert_eq!((p.direccion, p.mnemonico.as_str()), (0x0100, "LDI"));
    assert!(e.interrupciones_pendientes().is_empty());
}

#[test]
fn atender_apila_el_pc_en_little_endian_y_baja_el_sp() {
    let mut e = emulador();
    e.paso().unwrap(); // NOP: pc = 1
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    e.paso().unwrap();
    assert_eq!(e.estado().sp(), Some(0xEFFD));
    assert_eq!(e.memoria().leer(0xEFFD).unwrap(), 0x01);
    assert_eq!(e.memoria().leer(0xEFFE).unwrap(), 0x00);
}

#[test]
fn atender_inhabilita_las_interrupciones() {
    let mut e = emulador();
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    e.paso().unwrap();
    assert!(!e.estado().interrupciones_habilitadas());
}

#[test]
fn el_manejador_se_ejecuta_hasta_detenerse() {
    let mut e = emulador();
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("C").unwrap(), 0x99);
}

#[test]
fn gana_la_linea_de_menor_numero() {
    let mut e = emulador();
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(1).unwrap();
    e.levantar_interrupcion(0).unwrap();
    assert_eq!(e.interrupciones_pendientes(), [0, 1]);
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("C").unwrap(), 0x99);
    assert_eq!(e.estado().registro("D").unwrap(), 0);
    assert_eq!(e.interrupciones_pendientes(), [1]);
}

#[test]
fn la_segunda_linea_espera_a_que_se_habiliten_de_nuevo() {
    let mut e = emulador();
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    e.levantar_interrupcion(1).unwrap();
    // Se atiende la linea 0 y se ejecuta la primera instruccion de su manejador.
    e.paso().unwrap();
    assert_eq!(e.interrupciones_pendientes(), [1]);
    e.estado_mut().habilitar_interrupciones(true);
    let p = e.paso().unwrap().unwrap();
    assert_eq!(p.direccion, 0x0110);
}

#[test]
fn levantar_dos_veces_la_misma_linea_cuenta_una_vez() {
    let mut e = emulador();
    e.levantar_interrupcion(0).unwrap();
    e.levantar_interrupcion(0).unwrap();
    assert_eq!(e.interrupciones_pendientes(), [0]);
}

#[test]
fn en_big_endian_el_vector_y_el_pc_se_guardan_con_el_byte_alto_primero() {
    let mut m = ejemplo_8();
    m.endianness = Endianness::Big;
    let mut e = Emulador::nuevo(m).unwrap();
    e.cargar_programa(0x0000, &[0x00, 0xF0]).unwrap();
    e.cargar_programa(0x0100, &[0xF0]).unwrap();
    e.cargar_programa(0xEFF0, &[0x01, 0x00]).unwrap();
    e.paso().unwrap(); // pc = 1
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    let p = e.paso().unwrap().unwrap();
    assert_eq!(p.direccion, 0x0100);
    assert_eq!(e.memoria().leer(0xEFFD).unwrap(), 0x00);
    assert_eq!(e.memoria().leer(0xEFFE).unwrap(), 0x01);
}

#[test]
fn con_pila_ascendente_se_guarda_en_sp_y_sube() {
    let mut m = ejemplo_8();
    m.pila = SentidoPila::Ascendente;
    let mut e = emulador_de(m);
    e.estado_mut().fijar_sp(0x2000);
    e.estado_mut().habilitar_interrupciones(true);
    e.paso().unwrap();
    e.levantar_interrupcion(0).unwrap();
    e.paso().unwrap();
    assert_eq!(e.estado().sp(), Some(0x2002));
    assert_eq!(e.memoria().leer(0x2000).unwrap(), 0x01);
}

#[test]
fn sin_puntero_de_pila_no_se_puede_atender() {
    let mut m = ejemplo_8();
    m.registros.retain(|r| r.rol != RolRegistro::PunteroPila);
    let mut e = emulador_de(m);
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    assert_eq!(e.paso(), Err(ErrorEjecucion::InterrupcionSinPila { linea: 0 }));
}

#[test]
fn si_el_vector_cae_fuera_de_las_regiones_se_informa() {
    let mut m = ejemplo_8();
    m.interrupciones.as_mut().unwrap().vectores.insert(2, 0xF800);
    let mut e = emulador_de(m);
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(2).unwrap();
    assert!(matches!(e.paso(), Err(ErrorEjecucion::FueraDeRegion { direccion: 0xF800 })));
}

#[test]
fn si_la_pila_cae_en_solo_lectura_el_fallo_no_cambia_el_pc() {
    let mut e = emulador();
    e.estado_mut().fijar_sp(0x0100);
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    assert!(matches!(e.paso(), Err(ErrorEjecucion::EscrituraEnSoloLectura { .. })));
    assert_eq!(e.estado().pc(), 0);
}

#[test]
fn reiniciar_descarta_las_interrupciones_pendientes() {
    let mut e = emulador();
    e.levantar_interrupcion(0).unwrap();
    e.reiniciar();
    assert!(e.interrupciones_pendientes().is_empty());
}

#[test]
fn una_maquina_detenida_no_atiende_interrupciones() {
    let mut e = emulador();
    e.ejecutar(10).unwrap();
    e.estado_mut().habilitar_interrupciones(true);
    e.levantar_interrupcion(0).unwrap();
    assert_eq!(e.paso().unwrap(), None);
    assert_eq!(e.interrupciones_pendientes(), [0]);
}
