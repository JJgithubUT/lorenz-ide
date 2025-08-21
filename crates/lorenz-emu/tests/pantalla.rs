//! Pruebas de la pantalla: de la memoria de video a la imagen.

mod comun;

use comun::ejemplo_8;
use lorenz_emu::{Emulador, ErrorEjecucion};

fn emulador() -> Emulador {
    Emulador::nuevo(ejemplo_8()).unwrap()
}

#[test]
fn una_pantalla_en_blanco_tiene_todos_los_pixeles_apagados() {
    let img = emulador().pantalla("pantalla").unwrap();
    assert_eq!((img.ancho, img.alto), (64, 32));
    assert_eq!(img.pixeles.len(), 64 * 32);
    assert!(img.pixeles.iter().all(|p| *p == 0));
}

#[test]
fn el_bit_mas_significativo_de_la_primera_palabra_es_el_primer_pixel() {
    let mut e = emulador();
    e.memoria_mut().escribir(0xF000, 0b1000_0001).unwrap();
    let img = e.pantalla("pantalla").unwrap();
    assert_eq!(img.pixel(0, 0), Some(1));
    assert_eq!(img.pixel(1, 0), Some(0));
    assert_eq!(img.pixel(7, 0), Some(1));
    assert_eq!(img.pixel(8, 0), Some(0));
}

#[test]
fn la_segunda_fila_empieza_en_la_palabra_ocho() {
    let mut e = emulador();
    // 64 pixeles por fila / 8 pixeles por palabra = 8 palabras por fila.
    e.memoria_mut().escribir(0xF008, 0b0100_0000).unwrap();
    let img = e.pantalla("pantalla").unwrap();
    assert_eq!(img.pixel(1, 1), Some(1));
    assert_eq!(img.pixel(1, 0), Some(0));
}

#[test]
fn el_ultimo_pixel_esta_en_la_ultima_palabra() {
    let mut e = emulador();
    e.memoria_mut().escribir(0xF0FF, 0b0000_0001).unwrap();
    let img = e.pantalla("pantalla").unwrap();
    assert_eq!(img.pixel(63, 31), Some(1));
}

#[test]
fn un_pixel_fuera_de_la_imagen_no_existe() {
    let img = emulador().pantalla("pantalla").unwrap();
    assert_eq!(img.pixel(64, 0), None);
    assert_eq!(img.pixel(0, 32), None);
}

#[test]
fn un_programa_dibuja_en_la_pantalla() {
    // LDI A,0xFF ; ST A,[0xF000] ; HLT
    let mut e = emulador();
    e.cargar_programa(0, &[0x20, 0xFF, 0x50, 0x00, 0xF0, 0xF0]).unwrap();
    e.ejecutar(10).unwrap();
    let img = e.pantalla("pantalla").unwrap();
    assert!((0..8).all(|x| img.pixel(x, 0) == Some(1)));
    assert_eq!(img.pixel(8, 0), Some(0));
}

#[test]
fn el_texto_dibuja_almohadillas_y_puntos() {
    let mut e = emulador();
    e.memoria_mut().escribir(0xF000, 0b1010_0000).unwrap();
    let texto = e.pantalla("pantalla").unwrap().a_texto();
    let primera = texto.lines().next().unwrap();
    assert_eq!(primera.len(), 64);
    assert!(primera.starts_with("#.#....."));
    assert_eq!(texto.lines().count(), 32);
}

#[test]
fn un_dispositivo_inexistente_da_error() {
    assert_eq!(
        emulador().pantalla("monitor"),
        Err(ErrorEjecucion::DispositivoInexistente { nombre: "monitor".into() })
    );
}

#[test]
fn un_dispositivo_que_no_es_pantalla_da_error() {
    assert!(matches!(
        emulador().pantalla("teclado"),
        Err(ErrorEjecucion::DispositivoIncorrecto { esperado: "una pantalla", .. })
    ));
}

#[test]
fn indice8_usa_una_palabra_por_pixel() {
    let mut m = ejemplo_8();
    for d in &mut m.dispositivos {
        if let lorenz_arch::modelo::TipoDispositivo::Pantalla { formato, ancho, alto } = &mut d.tipo {
            *formato = "indice8".into();
            *ancho = 16;
            *alto = 4;
        }
    }
    let mut e = Emulador::nuevo(m).unwrap();
    e.memoria_mut().escribir(0xF000 + 17, 5).unwrap();
    let img = e.pantalla("pantalla").unwrap();
    assert_eq!(img.pixel(1, 1), Some(5));
    assert_eq!(img.pixel(0, 0), Some(0));
}

#[test]
fn un_formato_desconocido_da_error() {
    let mut m = ejemplo_8();
    for d in &mut m.dispositivos {
        if let lorenz_arch::modelo::TipoDispositivo::Pantalla { formato, .. } = &mut d.tipo {
            *formato = "rgb565".into();
        }
    }
    let e = Emulador::nuevo(m).unwrap();
    assert!(matches!(e.pantalla("pantalla"), Err(ErrorEjecucion::FormatoDePantallaNoSoportado { .. })));
}

#[test]
fn si_la_region_es_mas_chica_que_la_imagen_el_resto_queda_apagado() {
    let mut m = ejemplo_8();
    for d in &mut m.dispositivos {
        if let lorenz_arch::modelo::TipoDispositivo::Pantalla { ancho, alto, .. } = &mut d.tipo {
            *ancho = 64;
            *alto = 64;
        }
    }
    let mut e = Emulador::nuevo(m).unwrap();
    e.memoria_mut().escribir(0xF0FF, 1).unwrap();
    let img = e.pantalla("pantalla").unwrap();
    assert_eq!(img.pixeles.len(), 64 * 64);
    assert_eq!(img.pixel(63, 31), Some(1));
    assert_eq!(img.pixel(0, 40), Some(0));
}
