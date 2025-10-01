//! Pruebas del interprete de ordenes.

use lorenz_dbg::comandos::Sesion;
use lorenz_dbg::Depurador;
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

const PROGRAMA: &str = "\
inicio: LDI A, 1
        LDI B, 2
suma:   ADD A, B
        ST [0x2000], A
        HLT
";

fn sesion() -> Sesion {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", PROGRAMA))
        .unwrap();
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    let mut d = Depurador::nuevo(emu);
    d.con_fuente(&obj, &[("p.s".into(), PROGRAMA.into())]);
    Sesion::nueva(d)
}

fn di(s: &mut Sesion, orden: &str) -> String {
    s.ejecutar(orden).texto
}

#[test]
fn paso_muestra_la_siguiente_instruccion() {
    let mut s = sesion();
    let t = di(&mut s, "paso");
    assert!(t.contains("1 instruccion"), "{t}");
    assert!(t.contains("LDI B"), "{t}");
}

#[test]
fn formas_cortas() {
    let mut s = sesion();
    assert!(di(&mut s, "s 2").contains("2 instruccion"));
    assert!(di(&mut s, "r").contains("A = 01"));
}

#[test]
fn linea_vacia_repite() {
    let mut s = sesion();
    di(&mut s, "paso");
    let t = di(&mut s, "");
    assert!(t.contains("instruccion"), "{t}");
    assert_eq!(s.depurador.emulador().estado().registro("B").unwrap(), 2);
}

#[test]
fn ruptura_con_condicion_y_puntos() {
    let mut s = sesion();
    assert!(di(&mut s, "ruptura suma si A == 1").contains("ruptura 1"));
    assert!(di(&mut s, "puntos").contains("si A == 1"));
    assert!(di(&mut s, "continuar").contains("ruptura 1"));
}

#[test]
fn fijar_y_escribir() {
    let mut s = sesion();
    di(&mut s, "fijar A 0x7");
    di(&mut s, "escribir 0x2010 0xAB");
    assert_eq!(s.depurador.emulador().estado().registro("A").unwrap(), 7);
    assert!(di(&mut s, "volcar 0x2010 4").contains("ab"));
    assert!(di(&mut s, "fijar Z 1").contains("Z"));
}

#[test]
fn fijar_nombre_desconocido() {
    let mut s = sesion();
    assert!(di(&mut s, "fijar Q 1").starts_with("error:"));
}

#[test]
fn imprimir_expresiones() {
    let mut s = sesion();
    di(&mut s, "paso 2");
    assert!(di(&mut s, "imprimir A + B").starts_with("3 "));
    assert!(di(&mut s, "p mem[0]").contains('('));
}

#[test]
fn desensamblar_con_fuente_y_marca() {
    let mut s = sesion();
    let t = di(&mut s, "desensamblar inicio 3");
    assert!(t.contains("=>"), "{t}");
    assert!(t.contains("inicio:"), "{t}");
    assert!(t.contains("p.s:1"), "{t}");
}

#[test]
fn observar_y_borrar() {
    let mut s = sesion();
    assert!(di(&mut s, "observar 0x2000 escritura").contains("observacion 1"));
    assert!(di(&mut s, "continuar").contains("escritura de 0x2000"));
    di(&mut s, "borrar 1");
    assert!(di(&mut s, "puntos").contains("no hay puntos"));
}

#[test]
fn errores_en_español_y_la_sesion_sigue() {
    let mut s = sesion();
    assert!(di(&mut s, "bailar").contains("no existe"));
    assert!(di(&mut s, "volcar").contains("le falta"));
    assert!(di(&mut s, "registros extra").contains("le sobra"));
    assert!(di(&mut s, "borrar 9").contains("no hay ningun punto"));
    assert!(di(&mut s, "registros").contains("A = "));
}

#[test]
fn fin_termina() {
    let mut s = sesion();
    assert!(s.ejecutar("fin").salir);
    assert!(s.ejecutar("q").salir);
    assert!(!s.ejecutar("ayuda").salir);
}

#[test]
fn ayuda_lista_las_ordenes() {
    let mut s = sesion();
    let t = di(&mut s, "ayuda");
    for o in ["paso", "continuar", "ruptura", "observar", "volcar"] {
        assert!(t.contains(o), "{o}");
    }
}

#[test]
fn hasta_una_etiqueta() {
    let mut s = sesion();
    assert!(di(&mut s, "hasta suma").contains("se llego a"));
}

#[test]
fn puertos_a_mano() {
    let mut s = sesion();
    assert!(di(&mut s, "entrada 0x01 0x41").contains("en cola"));
    assert!(di(&mut s, "puerto 0x02").contains("-> 0x1"));
    assert!(di(&mut s, "puerto 0x01").contains("-> 0x41"));
    assert!(di(&mut s, "puerto 0x01").contains("-> 0x0"));
    assert!(di(&mut s, "puerto 0x77").starts_with("error:"));
}
