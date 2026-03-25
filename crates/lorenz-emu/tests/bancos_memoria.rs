//! Bancos de memoria conmutables por puerto (RF-ARQ-033).

use lorenz_emu::Emulador;

const MAQUINA: &str = r#"
[maquina]
nombre = "con-bancos-de-memoria"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[[registro]]
nombre = "A"
bits = 8

[[registro]]
nombre = "PC"
bits = 16
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
mnemonico = "ST"
sintaxis = "ST [{dir}]"
codigo = "0100 0000 | dir:8 | dir:8"
semantica = "mem[dir] = A"
[[instruccion.operando]]
nombre = "dir"
tipo = "direccion"

[[instruccion]]
mnemonico = "LD"
sintaxis = "LD [{dir}]"
codigo = "0101 0000 | dir:8 | dir:8"
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
desde = 0x0000
hasta = 0x7FFF
acceso = "rw"

[[region]]
nombre = "ventana"
desde = 0x8000
hasta = 0x80FF
acceso = "rw"
bancos = 3
selector_puerto = 0x10
"#;

const LDI: u64 = 0x20;
const ST: u64 = 0x40;
const LD: u64 = 0x50;
const OUT: u64 = 0x60;
const IN: u64 = 0x70;
const HLT: u64 = 0xF0;

fn correr(programa: &[u64]) -> Emulador {
    let mut e = Emulador::nuevo(lorenz_arch::cargar(MAQUINA).unwrap()).unwrap();
    e.cargar_programa(0, programa).unwrap();
    e.ejecutar(1000).unwrap();
    e
}

fn a(e: &Emulador) -> u64 {
    e.estado().registro("A").unwrap()
}

/// LDI valor ; ST [0x8001] ; LDI banco ; OUT 0x10
fn escribir_en(banco: u64, valor: u64) -> Vec<u64> {
    vec![LDI, banco, OUT, 0x10, LDI, valor, ST, 0x01, 0x80]
}

#[test]
fn cada_banco_guarda_sus_propios_datos_en_las_mismas_direcciones() {
    let mut p = escribir_en(0, 11);
    p.extend(escribir_en(1, 22));
    p.extend(escribir_en(2, 33));
    p.extend([LDI, 1, OUT, 0x10, LD, 0x01, 0x80, HLT]);
    let e = correr(&p);
    assert_eq!(a(&e), 22);
    assert_eq!(e.memoria().banco_activo("ventana"), Some(1));
    for (banco, esperado) in [(0, 11), (1, 22), (2, 33)] {
        assert_eq!(
            e.memoria().leer_en_banco("ventana", banco, 0x8001).unwrap(),
            esperado
        );
    }
}

#[test]
fn volver_a_un_banco_recupera_lo_que_tenia() {
    let mut p = escribir_en(0, 7);
    p.extend([LDI, 2, OUT, 0x10, LDI, 0, OUT, 0x10, LD, 0x01, 0x80, HLT]);
    assert_eq!(a(&correr(&p)), 7);
}

#[test]
fn un_banco_nuevo_esta_vacio() {
    let mut p = escribir_en(0, 9);
    p.extend([LDI, 1, OUT, 0x10, LD, 0x01, 0x80, HLT]);
    assert_eq!(a(&correr(&p)), 0);
}

#[test]
fn leer_el_puerto_selector_dice_el_banco_visible() {
    let e = correr(&[LDI, 2, OUT, 0x10, LDI, 0, IN, 0x10, HLT]);
    assert_eq!(a(&e), 2);
}

#[test]
fn un_valor_mayor_que_los_bancos_da_la_vuelta() {
    let e = correr(&[LDI, 5, OUT, 0x10, HLT]);
    assert_eq!(e.memoria().banco_activo("ventana"), Some(2));
}

#[test]
fn las_regiones_sin_bancos_no_tienen_banco_activo() {
    let e = correr(&[HLT]);
    assert_eq!(e.memoria().banco_activo("rom"), None);
    assert_eq!(e.memoria().banco_activo("no-existe"), None);
}

#[test]
fn la_instantanea_conserva_todos_los_bancos() {
    let mut p = escribir_en(0, 4);
    p.extend(escribir_en(1, 5));
    p.push(HLT);
    let mut e = correr(&p);
    let foto = e.instantanea();
    e.memoria_mut().seleccionar_banco(0x10, 0);
    e.memoria_mut().escribir(0x8001, 99).unwrap();
    e.restaurar(&foto).unwrap();
    assert_eq!(e.memoria().banco_activo("ventana"), Some(1));
    assert_eq!(e.memoria().leer(0x8001).unwrap(), 5);
    assert_eq!(e.memoria().leer_en_banco("ventana", 0, 0x8001).unwrap(), 4);
}

#[test]
fn la_suma_de_verificacion_cambia_con_el_banco_visible() {
    let mut p = escribir_en(0, 4);
    p.push(HLT);
    let mut e = correr(&p);
    let con_dato = e.memoria().suma_de_verificacion();
    e.memoria_mut().seleccionar_banco(0x10, 1);
    assert_ne!(e.memoria().suma_de_verificacion(), con_dato);
    e.memoria_mut().seleccionar_banco(0x10, 0);
    assert_eq!(e.memoria().suma_de_verificacion(), con_dato);
}

#[test]
fn con_seguimiento_las_lecturas_de_un_banco_sin_escribir_se_avisan() {
    let mut e = Emulador::nuevo(lorenz_arch::cargar(MAQUINA).unwrap()).unwrap();
    e.memoria_mut().activar_seguimiento();
    e.cargar_programa(0, &[LDI, 1, OUT, 0x10, LD, 0x01, 0x80, HLT])
        .unwrap();
    e.ejecutar(100).unwrap();
    assert_eq!(e.memoria().lecturas_sin_inicializar().len(), 1);
}
