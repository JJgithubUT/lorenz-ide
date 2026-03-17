//! Controlador de acceso directo a memoria (RF-CIC-022).

use lorenz_emu::{Emulador, TransferenciaDma};

const MAQUINA: &str = r#"
[maquina]
nombre = "con-dma"
formato = "0.1"
bits_palabra = 8
bits_direccion = 8
arranque = 0x00

[maquina.interrupciones]
vectores = { 0 = 0x38 }
habilitadas_al_arrancar = true

[[registro]]
nombre = "A"
bits = 8

[[registro]]
nombre = "PC"
bits = 8
rol = "pc"

[[instruccion]]
mnemonico = "LDI"
sintaxis = "LDI {imm}"
codigo = "0010 0000 | imm:8"
semantica = "A = imm"
[[instruccion.operando]]
nombre = "imm"
tipo = "inmediato"
signo = false
rango = [0, 255]

[[instruccion]]
mnemonico = "OUT"
sintaxis = "OUT {p}"
codigo = "0110 0000 | p:8"
semantica = "out[p] = A"
[[instruccion.operando]]
nombre = "p"
tipo = "puerto"

[[instruccion]]
mnemonico = "IN"
sintaxis = "IN {p}"
codigo = "0111 0000 | p:8"
semantica = "A = in[p]"
[[instruccion.operando]]
nombre = "p"
tipo = "puerto"

[[instruccion]]
mnemonico = "HLT"
sintaxis = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[region]]
nombre = "rom"
desde = 0x00
hasta = 0x3F
acceso = "r"

[[region]]
nombre = "ram"
desde = 0x40
hasta = 0xFF
acceso = "rw"

[[dispositivo]]
nombre = "dma0"
tipo = "dma"
puertos = { origen = 0x20, destino = 0x21, cuenta = 0x22, control = 0x23, estado = 0x24 }
"#;

const LDI: u64 = 0x20;
const OUT: u64 = 0x60;
const IN: u64 = 0x70;
const HLT: u64 = 0xF0;

fn maquina(extra: &str) -> Emulador {
    let texto = MAQUINA.replace("estado = 0x24 }", &format!("estado = 0x24 }}\n{extra}"));
    Emulador::nuevo(lorenz_arch::cargar(&texto).unwrap()).unwrap()
}

/// Programa que pide copiar `cuenta` palabras de `origen` a `destino`.
fn pedir(origen: u64, destino: u64, cuenta: u64) -> Vec<u64> {
    vec![
        LDI, origen, OUT, 0x20, LDI, destino, OUT, 0x21, LDI, cuenta, OUT, 0x22, LDI, 1, OUT, 0x23,
        HLT,
    ]
}

fn con_datos(e: &mut Emulador, desde: u64, datos: &[u64]) {
    for (k, v) in datos.iter().enumerate() {
        e.memoria_mut()
            .escribir_privilegiado(desde + k as u64, *v)
            .unwrap();
    }
}

fn celdas(e: &Emulador, desde: u64, n: u64) -> Vec<u64> {
    (0..n)
        .map(|k| e.memoria().leer_privilegiado(desde + k).unwrap())
        .collect()
}

#[test]
fn copia_las_palabras_sin_que_las_mueva_el_programa() {
    let mut e = maquina("");
    con_datos(&mut e, 0x40, &[1, 2, 3, 4]);
    e.cargar_programa(0, &pedir(0x40, 0x80, 4)).unwrap();
    e.ejecutar(100).unwrap();
    assert_eq!(celdas(&e, 0x80, 4), [1, 2, 3, 4]);
}

#[test]
fn registra_la_transferencia_para_el_modelo_de_ciclos() {
    let mut e = maquina("");
    con_datos(&mut e, 0x40, &[9, 9]);
    e.cargar_programa(0, &pedir(0x40, 0x80, 2)).unwrap();
    e.ejecutar(100).unwrap();
    assert_eq!(
        e.tomar_transferencias_dma(),
        [TransferenciaDma {
            dispositivo: "dma0".into(),
            origen: 0x40,
            destino: 0x80,
            palabras: 2,
            error: false,
        }]
    );
    assert!(e.tomar_transferencias_dma().is_empty());
}

#[test]
fn los_puertos_devuelven_lo_escrito_y_el_estado_dice_listo() {
    let mut e = maquina("");
    e.cargar_programa(0, &pedir(0x40, 0x80, 3)).unwrap();
    e.ejecutar(100).unwrap();
    assert_eq!(e.puertos_mut().leer(0x20).unwrap(), 0x40);
    assert_eq!(e.puertos_mut().leer(0x21).unwrap(), 0x80);
    assert_eq!(e.puertos_mut().leer(0x22).unwrap(), 3);
    assert_eq!(e.puertos_mut().leer(0x24).unwrap(), 1);
}

#[test]
fn un_destino_de_solo_lectura_falla_y_no_copia_nada() {
    let mut e = maquina("");
    con_datos(&mut e, 0x40, &[7, 7]);
    e.cargar_programa(0, &pedir(0x40, 0x10, 2)).unwrap();
    e.ejecutar(100).unwrap();
    assert_eq!(e.puertos_mut().leer(0x24).unwrap(), 0b11);
    let t = e.tomar_transferencias_dma();
    assert!(t[0].error && t[0].palabras == 0);
}

#[test]
fn una_transferencia_que_se_sale_de_la_memoria_falla() {
    let mut e = maquina("");
    e.cargar_programa(0, &pedir(0xFE, 0x80, 5)).unwrap();
    e.ejecutar(100).unwrap();
    assert_eq!(e.puertos_mut().leer(0x24).unwrap() & 0b10, 0b10);
}

#[test]
fn copiar_sobre_si_misma_conserva_el_orden_original() {
    let mut e = maquina("");
    con_datos(&mut e, 0x40, &[1, 2, 3, 4]);
    e.cargar_programa(0, &pedir(0x40, 0x41, 3)).unwrap();
    e.ejecutar(100).unwrap();
    assert_eq!(celdas(&e, 0x40, 4), [1, 1, 2, 3]);
}

#[test]
fn cuenta_cero_no_copia_ni_falla() {
    let mut e = maquina("");
    e.cargar_programa(0, &pedir(0x40, 0x80, 0)).unwrap();
    e.ejecutar(100).unwrap();
    assert_eq!(e.puertos_mut().leer(0x24).unwrap(), 1);
}

#[test]
fn escribir_control_sin_el_bit_cero_no_arranca() {
    let mut e = maquina("");
    con_datos(&mut e, 0x40, &[5]);
    let mut p = pedir(0x40, 0x80, 1);
    let n = p.len();
    p[n - 4] = 0;
    e.cargar_programa(0, &p).unwrap();
    e.ejecutar(100).unwrap();
    assert!(e.tomar_transferencias_dma().is_empty());
    assert_eq!(celdas(&e, 0x80, 1), [0]);
}

#[test]
fn al_terminar_levanta_su_linea_de_interrupcion() {
    let texto = MAQUINA.replace("estado = 0x24 }", "estado = 0x24 }\ninterrupcion = 0");
    let mut e = Emulador::nuevo(lorenz_arch::cargar(&texto).unwrap()).unwrap();
    e.cargar_programa(0, &pedir(0x40, 0x80, 1)).unwrap();
    for _ in 0..7 {
        e.paso().unwrap();
    }
    assert!(e.interrupciones_pendientes().is_empty());
    e.paso().unwrap(); // OUT del control: arranca y termina la transferencia
    assert_eq!(e.interrupciones_pendientes(), [0]);
}

#[test]
fn un_dma_sin_todos_sus_puertos_no_es_valido() {
    let texto = MAQUINA.replace("cuenta = 0x22, ", "");
    let e = lorenz_arch::cargar(&texto).unwrap_err();
    assert!(
        format!("{e:?}").contains("necesita el puerto 'cuenta'"),
        "{e:?}"
    );
}

#[test]
fn leer_el_puerto_de_control_da_cero() {
    let mut e = maquina("");
    e.cargar_programa(0, &[IN, 0x23, HLT]).unwrap();
    e.ejecutar(10).unwrap();
    assert_eq!(e.estado().registro("A").unwrap(), 0);
}
