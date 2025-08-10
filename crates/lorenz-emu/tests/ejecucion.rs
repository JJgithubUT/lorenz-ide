//! Pruebas de extremo a extremo: programas completos sobre la maquina de ejemplo de 8 bits.

mod comun;

use comun::ejemplo_8;
use lorenz_emu::{Emulador, ErrorEjecucion};

fn con_programa(palabras: &[u64]) -> Emulador {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.cargar_programa(0, palabras).unwrap();
    e
}

fn reg(e: &Emulador, nombre: &str) -> u64 {
    e.estado().registro(nombre).unwrap()
}

#[test]
fn un_programa_que_solo_se_detiene() {
    let mut e = con_programa(&[0xF0]);
    let r = e.ejecutar(10).unwrap();
    assert_eq!((r.instrucciones, r.ciclos), (1, 1));
    assert!(e.estado().detenida());
}

#[test]
fn ldi_carga_una_constante() {
    let mut e = con_programa(&[0x20, 42, 0xF0]);
    e.ejecutar(10).unwrap();
    assert_eq!(reg(&e, "A"), 42);
    assert_eq!(e.estado().pc(), 3);
}

#[test]
fn mov_copia_entre_registros() {
    // LDI C,7 ; MOV A,C ; HLT
    let mut e = con_programa(&[0x28, 7, 0b0001_0010, 0xF0]);
    e.ejecutar(10).unwrap();
    assert_eq!((reg(&e, "A"), reg(&e, "C")), (7, 7));
}

#[test]
fn suma_con_acarreo_y_ciclos() {
    // LDI A,200 ; LDI B,100 ; ADD A,B ; HLT
    let mut e = con_programa(&[0x20, 200, 0x24, 100, 0x31, 0xF0]);
    let r = e.ejecutar(10).unwrap();
    assert_eq!(reg(&e, "A"), 44);
    assert!(e.estado().bandera("C").unwrap());
    assert!(!e.estado().bandera("Z").unwrap());
    assert_eq!((r.instrucciones, r.ciclos), (4, 2 + 2 + 2 + 1));
}

#[test]
fn suma_que_da_cero_enciende_z() {
    let mut e = con_programa(&[0x20, 0, 0x24, 0, 0x31, 0xF0]);
    e.ejecutar(10).unwrap();
    assert!(e.estado().bandera("Z").unwrap());
    assert!(!e.estado().bandera("C").unwrap());
}

#[test]
fn guardar_y_cargar_de_la_ram() {
    // LDI A,0x42 ; ST A,[0x2000] ; LD B,[0x2000] ; HLT
    let mut e = con_programa(&[0x20, 0x42, 0x50, 0x00, 0x20, 0x44, 0x00, 0x20, 0xF0]);
    let r = e.ejecutar(10).unwrap();
    assert_eq!(e.memoria().leer(0x2000).unwrap(), 0x42);
    assert_eq!(reg(&e, "B"), 0x42);
    assert_eq!(r.ciclos, 2 + 4 + 4 + 1);
}

#[test]
fn jz_no_tomado_cuesta_dos_ciclos_y_sigue() {
    // JZ 0x0010 con Z apagada ; HLT
    let mut e = con_programa(&[0x60, 0x10, 0x00, 0xF0]);
    let r = e.ejecutar(10).unwrap();
    assert_eq!(r.ciclos, 2 + 1);
    assert_eq!(e.estado().pc(), 4);
}

#[test]
fn jz_tomado_cuesta_tres_ciclos_y_salta() {
    // ADD A,B (cero) ; JZ 0x0006 ; NOP ; HLT
    let mut e = con_programa(&[0x31, 0x60, 0x06, 0x00, 0x00, 0x00, 0xF0]);
    let r = e.ejecutar(10).unwrap();
    // ADD(2) + JZ tomado(3) + HLT(1); el NOP de 0x0004 y el de 0x0005 se saltan.
    assert_eq!(r.ciclos, 2 + 3 + 1);
    assert_eq!(r.instrucciones, 3);
}

#[test]
fn un_bucle_infinito_se_corta_con_el_limite() {
    // ADD A,B (Z) ; JZ 0x0001 (a si mismo)
    let mut e = con_programa(&[0x31, 0x60, 0x01, 0x00]);
    assert_eq!(e.ejecutar(100), Err(ErrorEjecucion::LimiteDeInstrucciones { limite: 100 }));
    assert_eq!(e.estado().instrucciones(), 100);
}

#[test]
fn el_limite_cuenta_por_llamada_y_se_puede_continuar() {
    let mut e = con_programa(&[0x31, 0x60, 0x01, 0x00]);
    assert!(e.ejecutar(10).is_err());
    assert!(e.ejecutar(10).is_err());
    assert_eq!(e.estado().instrucciones(), 20);
}

#[test]
fn un_limite_de_cero_no_ejecuta_nada() {
    let mut e = con_programa(&[0xF0]);
    assert_eq!(e.ejecutar(0), Err(ErrorEjecucion::LimiteDeInstrucciones { limite: 0 }));
    assert_eq!(e.estado().instrucciones(), 0);
}

#[test]
fn paso_a_paso_informa_de_cada_instruccion() {
    let mut e = con_programa(&[0x20, 5, 0xF0]);
    let p = e.paso().unwrap().unwrap();
    assert_eq!((p.direccion, p.mnemonico.as_str(), p.longitud, p.ciclos), (0, "LDI", 2, 2));
    assert!(!p.detenida && !p.salto);
    let p = e.paso().unwrap().unwrap();
    assert_eq!((p.direccion, p.mnemonico.as_str()), (2, "HLT"));
    assert!(p.detenida);
    assert_eq!(e.paso().unwrap(), None);
}

#[test]
fn escribir_en_la_rom_falla_y_deja_el_pc_en_la_instruccion() {
    // LDI A,1 ; ST A,[0x0010]
    let mut e = con_programa(&[0x20, 1, 0x50, 0x10, 0x00, 0xF0]);
    let err = e.ejecutar(10).unwrap_err();
    assert!(matches!(err, ErrorEjecucion::EscrituraEnSoloLectura { direccion: 0x10, .. }));
    assert_eq!(e.estado().pc(), 2);
}

#[test]
fn una_instruccion_invalida_se_informa_con_su_direccion() {
    let mut e = con_programa(&[0x00, 0x70]);
    let err = e.ejecutar(10).unwrap_err();
    assert_eq!(err, ErrorEjecucion::InstruccionInvalida { direccion: 1, palabra: 0x70 });
    assert_eq!(e.estado().pc(), 1);
}

#[test]
fn una_memoria_en_blanco_son_nop_hasta_salir_de_la_ultima_region() {
    // Rom, ram y video estan a cero (NOP): el pc avanza hasta 0xF100, donde no hay region.
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    let err = e.ejecutar(0x20000).unwrap_err();
    assert_eq!(err, ErrorEjecucion::FueraDeRegion { direccion: 0xF100 });
    assert_eq!(e.estado().instrucciones(), 0xF100);
    assert_eq!(e.estado().ciclos(), 0xF100);
}

#[test]
fn reiniciar_vuelve_al_arranque_pero_conserva_la_memoria() {
    let mut e = con_programa(&[0x20, 9, 0x50, 0x00, 0x20, 0xF0]);
    e.ejecutar(10).unwrap();
    e.reiniciar();
    assert_eq!(e.estado().pc(), 0);
    assert_eq!(reg(&e, "A"), 0);
    assert!(!e.estado().detenida());
    assert_eq!(e.estado().instrucciones(), 0);
    assert_eq!(e.memoria().leer(0x2000).unwrap(), 9);
}

#[test]
fn dos_ejecuciones_iguales_dan_el_mismo_resultado() {
    let programa = [0x20, 200, 0x24, 100, 0x31, 0x50, 0x00, 0x20, 0xF0];
    let mut a = con_programa(&programa);
    let mut b = con_programa(&programa);
    let ra = a.ejecutar(50).unwrap();
    let rb = b.ejecutar(50).unwrap();
    assert_eq!(ra, rb);
    assert_eq!(a.estado().registro("A").unwrap(), b.estado().registro("A").unwrap());
    assert_eq!(a.memoria().volcar(0x2000, 4).unwrap(), b.memoria().volcar(0x2000, 4).unwrap());
}

#[test]
fn escribir_en_la_pantalla_pasa_por_la_region_de_video() {
    // LDI A,1 ; ST A,[0xF000] ; HLT
    let mut e = con_programa(&[0x20, 1, 0x50, 0x00, 0xF0, 0xF0]);
    e.ejecutar(10).unwrap();
    assert_eq!(e.memoria().leer(0xF000).unwrap(), 1);
    assert_eq!(e.memoria().dispositivo_de(0xF000), Some("pantalla"));
}

#[test]
fn el_programa_puede_vivir_en_la_ram() {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.cargar_programa(0x2000, &[0x20, 3, 0xF0]).unwrap();
    e.estado_mut().fijar_pc(0x2000);
    e.ejecutar(10).unwrap();
    assert_eq!(reg(&e, "A"), 3);
}

#[test]
fn cargar_un_programa_fuera_de_las_regiones_falla_sin_escribir() {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    assert!(matches!(e.cargar_programa(0xF0FF, &[1, 2]), Err(ErrorEjecucion::BloqueFueraDeRegion { .. })));
    assert_eq!(e.memoria().leer(0xF0FF).unwrap(), 0);
}
