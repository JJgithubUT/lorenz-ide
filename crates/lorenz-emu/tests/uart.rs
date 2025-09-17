//! Pruebas de la UART: transmision, recepcion y estado.

mod comun;

use lorenz_emu::{Emulador, ErrorEjecucion};

/// Ejemplo de 8 bits con una UART (datos 0x10, estado 0x11, linea 1 compartida con el teclado se
/// cambia a la 0) y las instrucciones `OUT rs` e `IN rd` sobre su puerto de datos.
fn texto_maquina(linea: &str) -> String {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let con_e_s = texto.replacen(
        "[[instruccion]]\nmnemonico = \"HLT\"",
        r#"[[instruccion]]
mnemonico = "OUT"
sintaxis  = "OUT {rs}"
codigo    = "1001 rs:2 .."
semantica = "out[0x10] = rs"
ciclos    = 2
descripcion = "Transmite un byte por la UART."
  [[instruccion.operando]]
  nombre = "rs"
  tipo = "registro"
  de = ["A", "B", "C", "D"]

[[instruccion]]
mnemonico = "IN"
sintaxis  = "IN {rd}"
codigo    = "1000 rd:2 .."
semantica = "rd = in[0x10]"
ciclos    = 2
descripcion = "Recibe un byte de la UART."
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "B", "C", "D"]

[[instruccion]]
mnemonico = "INE"
sintaxis  = "INE {rd}"
codigo    = "1010 rd:2 .."
semantica = "rd = in[0x11]"
ciclos    = 2
descripcion = "Lee el estado de la UART."
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "B", "C", "D"]

[[instruccion]]
mnemonico = "HLT""#,
        1,
    );
    format!(
        "{con_e_s}\n[[dispositivo]]\nnombre = \"serie\"\ntipo = \"uart\"\npuertos = {{ datos = 0x10, estado = 0x11 }}\n{linea}"
    )
}

fn emulador(linea: &str) -> Emulador {
    let m = lorenz_arch::cargar(&texto_maquina(linea)).expect("la maquina con UART debe cargar");
    Emulador::nuevo(m).unwrap()
}

/// LDI r,c ; OUT r por cada caracter ; HLT
fn programa_que_escribe(texto: &str) -> Vec<u64> {
    let mut p = Vec::new();
    for b in texto.bytes() {
        p.extend([0x20, u64::from(b), 0b1001_0000]);
    }
    p.push(0xF0);
    p
}

#[test]
fn un_programa_transmite_texto_por_la_uart() {
    let mut e = emulador("");
    e.cargar_programa(0, &programa_que_escribe("Hola")).unwrap();
    e.ejecutar(100).unwrap();
    assert_eq!(e.texto_de_uart("serie").unwrap(), "Hola");
}

#[test]
fn sin_transmitir_el_texto_esta_vacio() {
    let mut e = emulador("");
    assert_eq!(e.texto_de_uart("serie").unwrap(), "");
}

#[test]
fn los_bytes_no_utf8_se_sustituyen() {
    let mut e = emulador("");
    e.cargar_programa(0, &programa_que_escribe("\u{7f}"))
        .unwrap();
    e.puertos_mut().escribir(0x10, 0xFF).unwrap();
    let texto = e.texto_de_uart("serie").unwrap();
    assert!(texto.contains('\u{fffd}'));
}

#[test]
fn recibir_bytes_los_deja_en_el_puerto_de_datos_en_orden() {
    let mut e = emulador("");
    e.recibir_en_uart("serie", b"ab").unwrap();
    assert_eq!(e.puertos_mut().leer(0x10).unwrap(), u64::from(b'a'));
    assert_eq!(e.puertos_mut().leer(0x10).unwrap(), u64::from(b'b'));
}

#[test]
fn el_estado_siempre_tiene_encendido_el_bit_de_transmision() {
    let mut e = emulador("");
    assert_eq!(e.puertos_mut().leer(0x11).unwrap(), 0b10);
}

#[test]
fn el_estado_enciende_el_bit_de_recepcion_si_hay_un_byte() {
    let mut e = emulador("");
    e.recibir_en_uart("serie", b"x").unwrap();
    assert_eq!(e.puertos_mut().leer(0x11).unwrap(), 0b11);
    e.puertos_mut().leer(0x10).unwrap();
    assert_eq!(e.puertos_mut().leer(0x11).unwrap(), 0b10);
}

#[test]
fn un_programa_lee_el_estado_y_el_byte_recibido() {
    let mut e = emulador("");
    // INE A ; IN B ; HLT
    e.cargar_programa(0, &[0b1010_0000, 0b1000_0100, 0xF0])
        .unwrap();
    e.recibir_en_uart("serie", b"z").unwrap();
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("A").unwrap(), 0b11);
    assert_eq!(e.estado().registro("B").unwrap(), u64::from(b'z'));
}

#[test]
fn con_linea_declarada_recibir_levanta_la_interrupcion() {
    let mut e = emulador("interrupcion = 0\n");
    e.recibir_en_uart("serie", b"x").unwrap();
    assert_eq!(e.interrupciones_pendientes(), [0]);
}

#[test]
fn recibir_sin_bytes_no_levanta_nada() {
    let mut e = emulador("interrupcion = 0\n");
    e.recibir_en_uart("serie", b"").unwrap();
    assert!(e.interrupciones_pendientes().is_empty());
}

#[test]
fn sin_linea_declarada_recibir_no_levanta_nada() {
    let mut e = emulador("");
    e.recibir_en_uart("serie", b"x").unwrap();
    assert!(e.interrupciones_pendientes().is_empty());
}

#[test]
fn un_dispositivo_inexistente_da_error() {
    assert_eq!(
        emulador("").recibir_en_uart("modem", b"x"),
        Err(ErrorEjecucion::DispositivoInexistente {
            nombre: "modem".into()
        })
    );
}

#[test]
fn un_dispositivo_que_no_es_uart_da_error() {
    let mut e = emulador("");
    assert!(matches!(
        e.recibir_en_uart("teclado", b"x"),
        Err(ErrorEjecucion::DispositivoIncorrecto {
            esperado: "una UART",
            ..
        })
    ));
    assert!(matches!(
        e.texto_de_uart("pantalla"),
        Err(ErrorEjecucion::DispositivoIncorrecto {
            esperado: "una UART",
            ..
        })
    ));
}

#[test]
fn reiniciar_borra_lo_transmitido_y_lo_recibido() {
    let mut e = emulador("");
    e.cargar_programa(0, &programa_que_escribe("A")).unwrap();
    e.ejecutar(10).unwrap();
    e.recibir_en_uart("serie", b"x").unwrap();
    e.reiniciar();
    assert_eq!(e.texto_de_uart("serie").unwrap(), "");
    assert_eq!(e.puertos_mut().leer(0x11).unwrap(), 0b10);
}

#[test]
fn el_teclado_no_se_ve_afectado_por_la_uart() {
    let mut e = emulador("");
    e.recibir_en_uart("serie", b"x").unwrap();
    assert_eq!(e.puertos_mut().leer(2).unwrap(), 0);
}
