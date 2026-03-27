//! Bancos de registros conmutables (RF-ARQ-012).

use lorenz_emu::Emulador;

const MAQUINA: &str = r#"
[maquina]
nombre = "con-bancos"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[[registro]]
nombre = "R1"
bits = 8
inicial = 5

[[registro]]
nombre = "R2"
bits = 8

[[registro]]
nombre = "SEL"
bits = 8

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[banco]]
selector = "SEL"
registros = ["R1", "R2"]
copias = 3

[[instruccion]]
mnemonico = "LDI"
sintaxis = "LDI {rd}, {imm}"
codigo = "0010 rd:2 .. | imm:8"
semantica = "rd = imm"
[[instruccion.operando]]
nombre = "rd"
tipo = "registro"
de = ["R1", "R2", "SEL"]
[[instruccion.operando]]
nombre = "imm"
tipo = "inmediato"
signo = false
rango = [0, 255]

[[instruccion]]
mnemonico = "HLT"
sintaxis = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
acceso = "rw"
"#;

const LDI_R1: u64 = 0x20;
const LDI_SEL: u64 = 0x28;
const HLT: u64 = 0xF0;

fn correr(programa: &[u64]) -> Emulador {
    let mut e = Emulador::nuevo(lorenz_arch::cargar(MAQUINA).unwrap()).unwrap();
    e.cargar_programa(0, programa).unwrap();
    e.ejecutar(100).unwrap();
    e
}

fn r(e: &Emulador, n: &str) -> u64 {
    e.estado().registro(n).unwrap()
}

#[test]
fn al_arrancar_se_ve_el_banco_cero_con_los_valores_iniciales() {
    let e = Emulador::nuevo(lorenz_arch::cargar(MAQUINA).unwrap()).unwrap();
    assert_eq!(r(&e, "R1"), 5);
    assert_eq!(e.estado().bancos_activos(), vec![0]);
}

#[test]
fn cada_banco_conserva_su_propio_valor() {
    // R1=10 ; SEL=1 ; R1=20 ; SEL=0 : R1 vuelve a ser 10.
    let e = correr(&[LDI_R1, 10, LDI_SEL, 1, LDI_R1, 20, LDI_SEL, 0, HLT]);
    assert_eq!(r(&e, "R1"), 10);
    assert_eq!(e.estado().registro_en_banco("R1", 1).unwrap(), 20);
}

#[test]
fn un_banco_nuevo_empieza_con_los_valores_iniciales() {
    let e = correr(&[LDI_R1, 10, LDI_SEL, 2, HLT]);
    assert_eq!(r(&e, "R1"), 5, "el banco 2 nunca se toco");
    assert_eq!(e.estado().registro_en_banco("R1", 0).unwrap(), 10);
}

#[test]
fn el_selector_y_los_registros_fuera_del_banco_no_cambian() {
    let e = correr(&[LDI_SEL, 1, HLT]);
    assert_eq!(r(&e, "SEL"), 1);
    let e = correr(&[LDI_SEL, 2, LDI_SEL, 0, HLT]);
    assert_eq!(r(&e, "SEL"), 0);
}

#[test]
fn un_selector_mayor_que_las_copias_da_la_vuelta() {
    let e = correr(&[LDI_R1, 7, LDI_SEL, 4, HLT]);
    // 4 % 3 = 1: se ve el banco 1, que esta sin tocar.
    assert_eq!(e.estado().bancos_activos(), vec![1]);
    assert_eq!(r(&e, "R1"), 5);
    assert_eq!(e.estado().registro_en_banco("R1", 0).unwrap(), 7);
}

#[test]
fn reiniciar_vuelve_al_banco_cero_y_borra_las_demas_copias() {
    let mut e = correr(&[LDI_R1, 10, LDI_SEL, 1, LDI_R1, 20, HLT]);
    e.reiniciar();
    assert_eq!(e.estado().bancos_activos(), vec![0]);
    assert_eq!(r(&e, "R1"), 5);
    assert_eq!(e.estado().registro_en_banco("R1", 1).unwrap(), 5);
}

#[test]
fn la_instantanea_conserva_todos_los_bancos() {
    let mut e = correr(&[LDI_R1, 10, LDI_SEL, 1, LDI_R1, 20, HLT]);
    let foto = e.instantanea();
    e.reiniciar();
    e.restaurar(&foto).unwrap();
    assert_eq!(r(&e, "R1"), 20);
    assert_eq!(e.estado().registro_en_banco("R1", 0).unwrap(), 10);
}

#[test]
fn pedir_una_copia_que_no_existe_es_un_error() {
    let e = correr(&[HLT]);
    assert!(e.estado().registro_en_banco("R1", 3).is_err());
}
