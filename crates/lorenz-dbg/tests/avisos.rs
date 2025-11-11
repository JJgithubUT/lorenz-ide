//! Avisos de pila desbordada y de lecturas sin inicializar (RF-DEP-039, RF-DEP-040, RF-DEP-041).

use lorenz_dbg::comandos::Sesion;
use lorenz_dbg::Depurador;
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

/// rica-8 con la rom escribible, para que la pila pueda crecer hacia abajo sin fallar al escribir.
fn depurador(programa: &str) -> Depurador {
    let t = std::fs::read_to_string("../lorenz-tst/tests/maquinas/rica-8.toml").unwrap();
    let t = t.replacen("acceso = \"r\"", "acceso = \"rw\"", 1);
    let m = lorenz_arch::cargar(&t).unwrap();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", programa))
        .unwrap_or_else(|e| panic!("{}", e[0]));
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    Depurador::nuevo(emu)
}

fn avisos_tras(d: &mut Depurador, pasos: u64) -> Vec<String> {
    d.paso(pasos);
    d.tomar_avisos()
}

#[test]
fn sin_activar_no_hay_avisos() {
    let mut d = depurador("ADD A, B\nHLT");
    assert!(avisos_tras(&mut d, 1).is_empty());
    assert!(!d.avisos_activos());
}

#[test]
fn leer_un_registro_sin_escribir_avisa_una_vez_por_instruccion() {
    let mut d = depurador("ADD A, B\nADD A, B\nHLT");
    d.avisar(true);
    let a = avisos_tras(&mut d, 1);
    assert_eq!(a.len(), 2, "A y B: {a:?}");
    assert!(a
        .iter()
        .any(|x| x.contains("registro B") && x.contains("0x0")));
    // El primer ADD escribio A; B sigue sin escribir y ahora lo lee otra instruccion.
    let b = avisos_tras(&mut d, 1);
    assert_eq!(b.len(), 1, "{b:?}");
    assert!(
        b[0].contains("registro B") && b[0].contains("0x1"),
        "{}",
        b[0]
    );
}

#[test]
fn escribir_antes_de_leer_no_avisa() {
    let mut d = depurador("LDI A, 1\nLDI B, 2\nADD A, B\nHLT");
    d.avisar(true);
    assert!(avisos_tras(&mut d, 3).is_empty());
}

#[test]
fn leer_una_celda_de_ram_sin_escribir_avisa() {
    let mut d = depurador("POP A\nHLT");
    d.emulador_mut()
        .estado_mut()
        .fijar_registro("SP", 0x2100)
        .unwrap();
    d.avisar(true);
    let a = avisos_tras(&mut d, 1);
    assert!(a.iter().any(|x| x.contains("celda 0x2100")), "{a:?}");
}

#[test]
fn escribir_la_celda_antes_evita_el_aviso() {
    let mut d = depurador("LDI A, 7\nPUSH A\nPOP B\nHLT");
    d.avisar(true);
    assert!(avisos_tras(&mut d, 3).is_empty());
}

#[test]
fn bajar_la_pila_de_su_region_avisa_del_desbordamiento() {
    let mut d = depurador("LDI A, 1\nPUSH A\nPUSH A\nHLT");
    d.emulador_mut()
        .estado_mut()
        .fijar_registro("SP", 0x2001)
        .unwrap();
    d.avisar(true);
    assert!(avisos_tras(&mut d, 2).is_empty(), "SP=0x2000 sigue dentro");
    let a = avisos_tras(&mut d, 1);
    assert_eq!(a.len(), 1, "{a:?}");
    assert!(
        a[0].contains("desbordamiento de pila") && a[0].contains("0x1fff"),
        "{}",
        a[0]
    );
}

#[test]
fn sacar_mas_de_lo_metido_avisa_de_que_se_vacio() {
    let mut d = depurador("LDI A, 1\nPUSH A\nPOP B\nPOP C\nHLT");
    d.avisar(true);
    d.paso(3);
    assert!(d.tomar_avisos().is_empty());
    let a = avisos_tras(&mut d, 1);
    assert!(a.iter().any(|x| x.contains("se vacio")), "{a:?}");
    assert!(a.iter().any(|x| x.contains("celda 0xefff")), "{a:?}");
}

#[test]
fn el_aviso_de_pila_no_se_repite_mientras_siga_fuera() {
    let mut d = depurador("LDI A, 1\nPUSH A\nPUSH A\nPUSH A\nHLT");
    d.emulador_mut()
        .estado_mut()
        .fijar_registro("SP", 0x2000)
        .unwrap();
    d.avisar(true);
    let a = avisos_tras(&mut d, 4);
    assert_eq!(
        a.iter().filter(|x| x.contains("desbordamiento")).count(),
        1,
        "{a:?}"
    );
}

#[test]
fn desactivar_los_avisos_los_apaga() {
    let mut d = depurador("ADD A, B\nADD C, D\nHLT");
    d.avisar(true);
    d.avisar(false);
    assert!(avisos_tras(&mut d, 2).is_empty());
}

#[test]
fn la_orden_avisar_se_muestra_al_parar() {
    let mut s = Sesion::nueva(depurador("ADD A, B\nHLT"));
    assert_eq!(s.ejecutar("avisar").texto, "avisos: apagados");
    assert!(s.ejecutar("avisar on").texto.contains("activos"));
    assert_eq!(s.ejecutar("avisar").texto, "avisos: activos");
    let t = s.ejecutar("paso").texto;
    assert!(
        t.contains("aviso: la instruccion en 0x0 leyo el registro"),
        "{t}"
    );
    assert_eq!(s.ejecutar("avisar off").texto, "avisos apagados");
}
