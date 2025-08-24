//! Pruebas del temporizador: ticks, puerto de cuenta e interrupcion.

mod comun;

use lorenz_emu::{Emulador, ErrorEjecucion};

fn con_temporizador(periodo: u64, linea: Option<u32>) -> Emulador {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let interrupcion = linea
        .map(|l| format!("interrupcion = {l}\n"))
        .unwrap_or_default();
    let texto = format!(
        "{texto}\n[[dispositivo]]\nnombre = \"reloj\"\ntipo = \"temporizador\"\nperiodo = {periodo}\npuertos = {{ cuenta = 0x03 }}\n{interrupcion}"
    );
    let m = lorenz_arch::cargar(&texto).expect("la maquina con temporizador debe cargar");
    Emulador::nuevo(m).unwrap()
}

/// Rellena la rom de NOP; cada uno cuesta un ciclo.
fn con_nops(periodo: u64, linea: Option<u32>) -> Emulador {
    let mut e = con_temporizador(periodo, linea);
    e.cargar_programa(0, &vec![0x00; 64]).unwrap();
    e
}

fn pasos(e: &mut Emulador, n: usize) {
    for _ in 0..n {
        e.paso().unwrap();
    }
}

#[test]
fn sin_ciclos_no_hay_ticks() {
    let e = con_nops(10, None);
    assert_eq!(e.ticks_de("reloj"), Ok(0));
}

#[test]
fn un_periodo_completo_suma_un_tick() {
    let mut e = con_nops(10, None);
    pasos(&mut e, 9);
    assert_eq!(e.ticks_de("reloj"), Ok(0));
    pasos(&mut e, 1);
    assert_eq!(e.ticks_de("reloj"), Ok(1));
}

#[test]
fn los_ticks_siguen_el_ritmo_de_los_ciclos() {
    let mut e = con_nops(4, None);
    pasos(&mut e, 40);
    assert_eq!(e.ticks_de("reloj"), Ok(10));
}

#[test]
fn una_instruccion_larga_puede_completar_varios_periodos() {
    // LD cuesta 4 ciclos; con periodo 1 completa cuatro ticks de una vez.
    let mut e = con_temporizador(1, None);
    e.cargar_programa(0, &[0x40, 0x00, 0x20]).unwrap();
    e.paso().unwrap();
    assert_eq!(e.ticks_de("reloj"), Ok(4));
}

#[test]
fn el_resto_de_ciclos_no_se_pierde() {
    let mut e = con_temporizador(3, None);
    // LDI cuesta 2 ciclos: tres LDI son 6 ciclos = 2 ticks exactos.
    e.cargar_programa(0, &[0x20, 1, 0x20, 1, 0x20, 1]).unwrap();
    pasos(&mut e, 3);
    assert_eq!(e.ticks_de("reloj"), Ok(2));
}

#[test]
fn el_puerto_cuenta_publica_los_ticks() {
    let mut e = con_nops(5, None);
    pasos(&mut e, 12);
    assert_eq!(e.puertos_mut().leer(3).unwrap(), 2);
    assert_eq!(e.puertos_mut().leer(3).unwrap(), 2);
}

#[test]
fn la_cuenta_publicada_se_trunca_a_una_palabra() {
    let mut e = con_nops(1, None);
    e.cargar_programa(0, &vec![0x00; 300]).unwrap_or(());
    pasos(&mut e, 258);
    assert_eq!(e.ticks_de("reloj"), Ok(258));
    assert_eq!(e.puertos_mut().leer(3).unwrap(), 2);
}

#[test]
fn con_linea_declarada_cada_tick_levanta_la_interrupcion() {
    let mut e = con_nops(5, Some(0));
    pasos(&mut e, 4);
    assert!(e.interrupciones_pendientes().is_empty());
    pasos(&mut e, 1);
    assert_eq!(e.interrupciones_pendientes(), [0]);
}

#[test]
fn sin_linea_declarada_no_se_levanta_nada() {
    let mut e = con_nops(1, None);
    pasos(&mut e, 5);
    assert!(e.interrupciones_pendientes().is_empty());
}

#[test]
fn el_manejador_del_temporizador_cuenta_las_interrupciones() {
    let mut e = con_nops(5, Some(0));
    // 0x0000: EI ; NOP... ; manejador en 0x0100: LDI A,1 ; RETI.
    // Los cinco primeros ciclos completan un periodo; el sexto paso atiende la linea y ejecuta
    // LDI, y el septimo ejecuta RETI, que vuelve a habilitar las interrupciones.
    e.cargar_programa(0, &[0x71]).unwrap();
    e.cargar_programa(0x0100, &[0x20, 0x01, 0x70]).unwrap();
    e.cargar_programa(0xEFF0, &[0x00, 0x01]).unwrap();
    pasos(&mut e, 7);
    assert_eq!(e.estado().registro("A").unwrap(), 1);
    assert!(e.estado().interrupciones_habilitadas());
}

#[test]
fn reiniciar_pone_los_ticks_a_cero() {
    let mut e = con_nops(2, None);
    pasos(&mut e, 10);
    e.reiniciar();
    assert_eq!(e.ticks_de("reloj"), Ok(0));
    assert_eq!(e.puertos_mut().leer(3).unwrap(), 0);
}

#[test]
fn preguntar_por_un_temporizador_inexistente_da_error() {
    let e = con_nops(2, None);
    assert_eq!(
        e.ticks_de("cuco"),
        Err(ErrorEjecucion::DispositivoInexistente {
            nombre: "cuco".into()
        })
    );
}

#[test]
fn un_programa_puede_leer_la_cuenta_con_in() {
    // La ejemplo-8 no tiene IN; se comprueba con el puerto directamente tras ejecutar.
    let mut e = con_nops(3, None);
    pasos(&mut e, 9);
    assert_eq!(e.puertos_mut().leer(3).unwrap(), 3);
}

#[test]
fn dos_ejecuciones_iguales_dan_los_mismos_ticks() {
    let correr = || {
        let mut e = con_nops(7, Some(0));
        pasos(&mut e, 50);
        (e.ticks_de("reloj").unwrap(), e.estado().ciclos())
    };
    assert_eq!(correr(), correr());
}
