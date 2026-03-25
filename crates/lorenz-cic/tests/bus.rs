//! Contencion del bus y controlador DMA en el modelo de ciclos (RF-CIC-021, RF-CIC-022).

use lorenz_cic::pipeline::Bus;
use lorenz_cic::{cargar_config, Simulador};
use lorenz_emu::Emulador;

const MAQUINA: &str = r#"
[maquina]
nombre = "con-dma"
formato = "0.1"
bits_palabra = 8
bits_direccion = 8
arranque = 0x00

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
mnemonico = "LD"
sintaxis = "LD [{dir}]"
codigo = "0101 0000 | dir:8"
semantica = "A = mem[dir]"
[[instruccion.operando]]
nombre = "dir"
tipo = "direccion"

[[instruccion]]
mnemonico = "OUT"
sintaxis = "OUT {p}"
codigo = "0110 0000 | p:8"
semantica = "out[p] = A"
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
latencia = 2

[[dispositivo]]
nombre = "dma0"
tipo = "dma"
puertos = { origen = 0x20, destino = 0x21, cuenta = 0x22, control = 0x23, estado = 0x24 }
"#;

const LDI: u8 = 0x20;
const LD: u8 = 0x50;
const OUT: u8 = 0x60;
const HLT: u8 = 0xF0;
const PIPE: &str = "[pipeline]\netapas = [\"IF\",\"ID\",\"EX\",\"MEM\",\"WB\"]\n";
const BUS: &str = "[bus]\ncompartido = true\n";

fn simulador(extra: &str, programa: &[u8]) -> Simulador {
    let texto = format!("{MAQUINA}\n{extra}\n");
    let m = lorenz_arch::cargar(&texto).unwrap();
    let cfg = cargar_config(&texto).unwrap();
    let mut e = Emulador::nuevo(m).unwrap();
    let palabras: Vec<u64> = programa.iter().map(|b| u64::from(*b)).collect();
    e.cargar_programa(0, &palabras).unwrap();
    Simulador::nuevo(e, cfg).unwrap()
}

/// Pide al DMA copiar `cuenta` palabras de `origen` a `destino`.
fn dma(origen: u8, destino: u8, cuenta: u8) -> Vec<u8> {
    vec![
        LDI, origen, OUT, 0x20, LDI, destino, OUT, 0x21, LDI, cuenta, OUT, 0x22, LDI, 1, OUT, 0x23,
    ]
}

/// Lee `n` veces una celda de la ram, que tiene latencia 2.
fn lecturas(n: usize) -> Vec<u8> {
    [LD, 0x50].repeat(n)
}

fn correr(extra: &str, mut programa: Vec<u8>) -> lorenz_cic::sim::Metricas {
    programa.push(HLT);
    let mut s = simulador(extra, &programa);
    s.ejecutar(10_000).unwrap();
    s.metricas()
}

// --- el bus como recurso ---------------------------------------------------------------------

#[test]
fn el_bus_libre_atiende_al_instante() {
    let mut b = Bus::default();
    assert_eq!(b.reservar(5, 3), 5);
    assert_eq!(b.ciclos_ocupado(), 3);
}

#[test]
fn una_peticion_que_choca_espera_a_que_acabe_la_anterior() {
    let mut b = Bus::default();
    b.reservar(5, 4);
    assert_eq!(b.reservar(6, 2), 9);
}

#[test]
fn una_peticion_toma_el_primer_hueco_en_que_cabe() {
    let mut b = Bus::default();
    b.reservar(0, 2); // 0..2
    b.reservar(5, 2); // 5..7
    assert_eq!(b.reservar(1, 3), 2, "cabe en 2..5");
    assert_eq!(b.reservar(1, 3), 7, "ya no hay hueco de 3 antes del 7");
}

#[test]
fn reservar_cero_ciclos_no_ocupa_nada() {
    let mut b = Bus::default();
    b.reservar(0, 5);
    assert_eq!(b.reservar(2, 0), 2);
    assert_eq!(b.ciclos_ocupado(), 5);
}

// --- bus compartido entre busqueda y datos -------------------------------------------------------

#[test]
fn sin_bus_compartido_no_hay_esperas_por_el_bus() {
    let m = correr(PIPE, lecturas(6));
    assert_eq!(m.esperas_bus, 0);
}

#[test]
fn con_bus_compartido_las_lecturas_estorban_a_la_busqueda() {
    let sin = correr(PIPE, lecturas(6));
    let con = correr(&format!("{PIPE}{BUS}"), lecturas(6));
    assert!(con.esperas_bus > 0, "{con:?}");
    assert!(con.ciclos > sin.ciclos, "{con:?} {sin:?}");
}

#[test]
fn el_bus_compartido_no_cambia_el_resultado_del_programa() {
    let mut a = simulador(PIPE, &[LDI, 7, HLT]);
    let mut b = simulador(&format!("{PIPE}{BUS}"), &[LDI, 7, HLT]);
    a.ejecutar(100).unwrap();
    b.ejecutar(100).unwrap();
    assert_eq!(
        a.emulador().estado().registro("A").unwrap(),
        b.emulador().estado().registro("A").unwrap()
    );
}

#[test]
fn la_vista_marca_la_espera_por_el_bus() {
    let mut s = simulador(
        &format!("{PIPE}{BUS}"),
        &[LD, 0x50, LD, 0x50, LD, 0x50, HLT],
    );
    s.ejecutar(100).unwrap();
    let hay = (0..s.ciclos()).any(|c| s.vista(c).unwrap().texto().contains("espera bus"));
    assert!(hay);
}

// --- DMA ------------------------------------------------------------------------------------------

#[test]
fn la_copia_del_dma_cuesta_lectura_y_escritura_de_cada_palabra() {
    let con = correr("", [dma(0x40, 0x80, 4), vec![]].concat());
    let sin = correr("", [dma(0x40, 0x80, 0), vec![]].concat());
    // Cada palabra: leer (ram, 2) y escribir (ram, 2).
    assert_eq!(con.ciclos_de_memoria - sin.ciclos_de_memoria, 4 * (2 + 2));
}

#[test]
fn el_dma_no_frena_al_procesador_si_el_bus_no_es_compartido() {
    let largo = correr(PIPE, [dma(0x40, 0x80, 30), lecturas(4)].concat());
    let corto = correr(PIPE, [dma(0x40, 0x80, 1), lecturas(4)].concat());
    assert_eq!(largo.ciclos, corto.ciclos);
}

#[test]
fn con_bus_compartido_un_dma_largo_retrasa_los_accesos_del_procesador() {
    let extra = format!("{PIPE}{BUS}");
    let largo = correr(&extra, [dma(0x40, 0x80, 30), lecturas(4)].concat());
    let corto = correr(&extra, [dma(0x40, 0x80, 1), lecturas(4)].concat());
    assert!(largo.ciclos > corto.ciclos + 50, "{largo:?} {corto:?}");
    assert!(largo.esperas_bus > corto.esperas_bus);
}

#[test]
fn el_dma_cuenta_en_el_uso_del_bus() {
    let m = correr(PIPE, dma(0x40, 0x80, 10));
    assert!(m.ciclos_de_memoria >= 40, "{m:?}");
    assert!(m.uso_del_bus().unwrap() > 0.0);
}

#[test]
fn un_bus_compartido_sin_pipeline_se_rechaza() {
    let e = cargar_config(&format!("{MAQUINA}\n{BUS}")).unwrap_err();
    assert!(e.contains("necesita un [pipeline]"), "{e}");
}
