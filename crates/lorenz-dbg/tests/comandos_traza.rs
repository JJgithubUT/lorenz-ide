//! Las ordenes de traza, retroceso, preguntas y fallos (RF-DEP-019 a RF-DEP-031).

use lorenz_dbg::comandos::Sesion;
use lorenz_dbg::Depurador;
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

const PROGRAMA: &str = "\
        LDI A, 1
        LDI B, 2
        ADD A, B
        ST [0x2000], A
        LDI A, 9
        ST [0x2000], A
        HLT
";

fn sesion_con(programa: &str) -> Sesion {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", programa))
        .unwrap();
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    let mut d = Depurador::nuevo(emu);
    d.con_fuente(&obj, &[("p.s".into(), programa.into())]);
    Sesion::nueva(d)
}

fn sesion() -> Sesion {
    sesion_con(PROGRAMA)
}

fn di(s: &mut Sesion, orden: &str) -> String {
    s.ejecutar(orden).texto
}

fn temporal(nombre: &str) -> String {
    let dir = std::env::temp_dir().join(format!("lorenz-dbg-cmd-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(nombre).to_string_lossy().into_owned()
}

#[test]
fn estado_de_la_traza() {
    let mut s = sesion();
    assert!(di(&mut s, "traza").contains("apagada"));
    assert!(di(&mut s, "traza on").contains("grabando"));
    di(&mut s, "paso 3");
    assert!(di(&mut s, "traza").contains("3 pasos"));
    assert!(di(&mut s, "traza off").contains("descartada"));
    assert!(di(&mut s, "traza").contains("apagada"));
}

#[test]
fn ver_la_traza() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "continuar");
    let t = di(&mut s, "traza ver 3");
    assert_eq!(t.lines().count(), 3, "{t}");
    assert!(t.contains("HLT"), "{t}");
    let t = di(&mut s, "traza ver 100");
    assert!(t.contains("LDI") && t.contains("A 0x0->0x1"), "{t}");
}

#[test]
fn atras_y_adelante() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "paso 4");
    let t = di(&mut s, "atras");
    assert!(t.contains("instante 3 de 4 (pasado)"), "{t}");
    assert!(t.contains("ST"), "{t}");
    let t = di(&mut s, "atras 2");
    assert!(t.contains("instante 1 de 4"), "{t}");
    let t = di(&mut s, "adelante 10");
    assert!(t.contains("instante 4 de 4"), "{t}");
    assert!(!t.contains("pasado"), "{t}");
}

#[test]
fn registros_en_el_pasado() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "continuar");
    di(&mut s, "instante 1");
    let r = di(&mut s, "registros");
    assert!(r.contains("A = 01"), "{r}");
    assert!(r.contains("B = 00"), "{r}");
    assert!(di(&mut s, "volcar 0x2000 1").contains("00"));
}

#[test]
fn instante_sin_argumento_solo_informa() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "paso 2");
    assert!(di(&mut s, "instante").contains("instante 2 de 2"));
    assert!(di(&mut s, "instante 9").starts_with("error:"));
}

#[test]
fn quien_escribio_responde_y_salta() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "continuar");
    let t = di(&mut s, "quien-escribio 0x2000");
    assert!(t.contains("la escribio ST"), "{t}");
    assert!(t.contains("instante 5"), "{t}");
    assert!(t.contains("0x3 -> 0x9"), "{t}");
    assert_eq!(s.depurador.instante(), 5);
    let t = di(&mut s, "quien-escribio 0x2000");
    assert!(t.contains("instante 3"), "{t}");
    let t = di(&mut s, "quien-escribio 0x2000");
    assert!(t.contains("nadie escribio"), "{t}");
}

#[test]
fn quien_cambio_un_registro() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "continuar");
    let t = di(&mut s, "quien-cambio B");
    assert!(t.contains("lo cambio LDI"), "{t}");
    assert!(t.contains("instante 1"), "{t}");
    assert!(di(&mut s, "quien-cambio Q").starts_with("error:"));
}

#[test]
fn buscar_una_condicion() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "continuar");
    let t = di(&mut s, "buscar A == 9 && mem[0x2000] == 3");
    assert!(t.contains("instante 5"), "{t}");
    assert_eq!(s.depurador.instante(), 5);
    assert!(di(&mut s, "buscar A == 77").contains("no se cumple"));
    assert!(di(&mut s, "buscar").contains("le falta"));
}

#[test]
fn sin_traza_las_preguntas_avisan() {
    let mut s = sesion();
    for o in [
        "atras",
        "quien-escribio 0x2000",
        "quien-cambio A",
        "buscar A == 1",
        "traza ver",
    ] {
        let t = di(&mut s, o);
        assert!(t.contains("traza"), "{o}: {t}");
    }
}

#[test]
fn la_traza_llena_se_avisa_al_ejecutar() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "traza limite 2");
    let t = di(&mut s, "paso 3");
    assert!(t.contains("aviso: la traza alcanzo su limite de 2"), "{t}");
    assert!(!di(&mut s, "paso").contains("aviso"));
}

#[test]
fn exportar_por_extension() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "continuar");
    let csv = temporal("t.csv");
    assert!(di(&mut s, &format!("traza exportar {csv}")).contains("7 pasos"));
    assert!(std::fs::read_to_string(&csv)
        .unwrap()
        .starts_with("instante,pc"));
    let js = temporal("t.json");
    di(&mut s, &format!("traza exportar {js}"));
    assert!(std::fs::read_to_string(&js)
        .unwrap()
        .contains("lorenz-traza-0.1"));
    assert!(di(&mut s, &format!("traza exportar {}", temporal("t.txt"))).starts_with("error:"));
}

#[test]
fn guardar_y_reproducir_un_fallo() {
    let mut s = sesion_con("LDI A, 5\nST [0x0010], A\nHLT\n");
    di(&mut s, "traza on");
    assert!(di(&mut s, "continuar").contains("fallo en"));
    let ruta = temporal("fallo.toml");
    let t = di(&mut s, &format!("fallo guardar {ruta}"));
    assert!(t.contains("1 pasos") && t.contains("solo lectura"), "{t}");
    let mut otra = sesion_con("HLT\n");
    let t = di(&mut otra, &format!("fallo reproducir {ruta}"));
    assert!(t.contains("reproducido: 1 pasos"), "{t}");
    assert_eq!(otra.depurador.emulador().estado().registro("A").unwrap(), 5);
}

#[test]
fn reproducir_un_archivo_que_no_existe() {
    let mut s = sesion();
    let t = di(&mut s, "fallo reproducir /no/existe.toml");
    assert!(t.starts_with("error:"), "{t}");
}

#[test]
fn fijar_con_traza_queda_en_la_historia() {
    let mut s = sesion();
    di(&mut s, "traza on");
    di(&mut s, "paso 1");
    di(&mut s, "fijar B 0x40");
    di(&mut s, "paso 1");
    assert_eq!(s.depurador.traza().unwrap().intervenciones().len(), 1);
}

#[test]
fn la_pantalla_se_dibuja_en_cada_instante() {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    assert!(t.contains("pantalla"));
    let mut s = sesion();
    let dibujo = di(&mut s, "pantalla");
    assert!(dibujo.contains('.'), "{dibujo}");
}

#[test]
fn ayuda_menciona_la_traza() {
    let mut s = sesion();
    let t = di(&mut s, "ayuda");
    for o in [
        "traza on",
        "atras",
        "quien-escribio",
        "buscar",
        "fallo guardar",
    ] {
        assert!(t.contains(o), "{o}");
    }
}
