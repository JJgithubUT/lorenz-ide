//! Pruebas del teclado: puertos, estado e interrupcion.

mod comun;

use comun::ejemplo_8;
use lorenz_emu::{Emulador, ErrorEjecucion};

fn emulador() -> Emulador {
    Emulador::nuevo(ejemplo_8()).unwrap()
}

#[test]
fn pulsar_una_tecla_la_deja_en_el_puerto_de_datos() {
    let mut e = emulador();
    e.pulsar_tecla("teclado", 0x41).unwrap();
    assert_eq!(e.puertos_mut().leer(1).unwrap(), 0x41);
}

#[test]
fn pulsar_una_tecla_cambia_el_estado_a_uno() {
    let mut e = emulador();
    assert_eq!(e.puertos_mut().leer(2).unwrap(), 0);
    e.pulsar_tecla("teclado", 0x41).unwrap();
    assert_eq!(e.puertos_mut().leer(2).unwrap(), 1);
}

#[test]
fn las_teclas_salen_en_el_orden_en_que_se_pulsaron() {
    let mut e = emulador();
    for t in [0x31, 0x32, 0x33] {
        e.pulsar_tecla("teclado", t).unwrap();
    }
    let leidas: Vec<u64> = (0..3).map(|_| e.puertos_mut().leer(1).unwrap()).collect();
    assert_eq!(leidas, [0x31, 0x32, 0x33]);
}

#[test]
fn pulsar_una_tecla_levanta_la_linea_del_teclado() {
    let mut e = emulador();
    e.pulsar_tecla("teclado", 0x41).unwrap();
    assert_eq!(e.interrupciones_pendientes(), [1]);
}

#[test]
fn un_teclado_sin_linea_no_levanta_interrupciones() {
    let mut m = ejemplo_8();
    for d in &mut m.dispositivos {
        d.interrupcion = None;
    }
    let mut e = Emulador::nuevo(m).unwrap();
    e.pulsar_tecla("teclado", 0x41).unwrap();
    assert!(e.interrupciones_pendientes().is_empty());
}

#[test]
fn una_tecla_se_trunca_al_ancho_de_palabra() {
    let mut e = emulador();
    e.pulsar_tecla("teclado", 0x141).unwrap();
    assert_eq!(e.puertos_mut().leer(1).unwrap(), 0x41);
}

#[test]
fn pulsar_en_un_dispositivo_inexistente_da_error() {
    assert_eq!(
        emulador().pulsar_tecla("raton", 1),
        Err(ErrorEjecucion::DispositivoInexistente {
            nombre: "raton".into()
        })
    );
}

#[test]
fn pulsar_en_un_dispositivo_que_no_es_teclado_da_error() {
    assert!(matches!(
        emulador().pulsar_tecla("pantalla", 1),
        Err(ErrorEjecucion::DispositivoIncorrecto {
            esperado: "un teclado",
            ..
        })
    ));
}

#[test]
fn reiniciar_vacia_las_teclas_pendientes() {
    let mut e = emulador();
    e.pulsar_tecla("teclado", 0x41).unwrap();
    e.reiniciar();
    assert_eq!(e.puertos_mut().leer(2).unwrap(), 0);
    assert!(e.interrupciones_pendientes().is_empty());
}

/// Un manejador de la linea 1 que lee la tecla y la guarda en la ram, con la maquina de 8 bits.
///
/// La ejemplo-8 no tiene instruccion de entrada, asi que el manejador se prueba con una maquina
/// derivada que anade `IN`.
#[test]
fn un_manejador_lee_la_tecla_por_interrupcion() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let con_in = texto.replacen(
        "[[instruccion]]\nmnemonico = \"HLT\"",
        r#"[[instruccion]]
mnemonico = "IN"
sintaxis  = "IN {rd}"
codigo    = "1000 rd:2 .."
semantica = "rd = in[1]"
ciclos    = 2
descripcion = "Lee el puerto de datos del teclado."
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "B", "C", "D"]

[[instruccion]]
mnemonico = "HLT""#,
        1,
    );
    let m = lorenz_arch::cargar(&con_in).expect("la maquina con IN debe cargar");
    let mut e = Emulador::nuevo(m).unwrap();
    // 0x0000: EI ; NOP ; NOP ; HLT
    e.cargar_programa(0x0000, &[0x71, 0x00, 0x00, 0xF0])
        .unwrap();
    // 0x0100: IN B ; ST B,[0x2000] ; RETI
    e.cargar_programa(0x0100, &[0b1000_0100, 0x54, 0x00, 0x20, 0x70])
        .unwrap();
    e.cargar_programa(0xEFF2, &[0x00, 0x01]).unwrap();
    e.paso().unwrap(); // EI
    e.pulsar_tecla("teclado", b'k' as u64).unwrap();
    e.ejecutar(20).unwrap();
    assert_eq!(e.memoria().leer(0x2000).unwrap(), b'k' as u64);
    assert_eq!(e.estado().registro("B").unwrap(), b'k' as u64);
    assert_eq!(e.puertos_mut().leer(2).unwrap(), 0);
}
