//! `salir` con subrutinas reales: CALL y RET en una maquina que los tiene (RF-DEP-003).

use lorenz_dbg::{Depurador, Parada};
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

const PROGRAMA: &str = "\
        CALL externa
        LDI A, 9
        HLT
externa: NOP
        CALL interna
        NOP
        RET
interna: NOP
        NOP
        RET
";

fn depurador() -> Depurador {
    let t = std::fs::read_to_string("../lorenz-tst/tests/maquinas/rica-8.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("sub.s", PROGRAMA))
        .unwrap_or_else(|e| panic!("{}", e[0]));
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    let mut d = Depurador::nuevo(emu);
    d.con_fuente(&obj, &[("sub.s".to_string(), PROGRAMA.to_string())]);
    d
}

fn pc(d: &Depurador) -> u64 {
    d.emulador().estado().pc()
}

#[test]
fn salir_de_una_subrutina_deja_el_pc_tras_la_llamada() {
    let mut d = depurador();
    let tras_la_llamada = {
        d.paso(1); // CALL externa
        let dentro = pc(&d);
        assert_ne!(dentro, 0);
        dentro
    };
    assert_eq!(d.salir(100).unwrap(), Parada::Retorno);
    assert!(
        pc(&d) < tras_la_llamada,
        "volvio a la direccion siguiente al CALL"
    );
    d.paso(1);
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 9);
}

#[test]
fn salir_ignora_las_llamadas_anidadas() {
    let mut d = depurador();
    d.paso(2); // CALL externa; NOP
               // Dentro de `externa`: la llamada a `interna` y su RET no cuentan como la salida.
    assert_eq!(d.salir(100).unwrap(), Parada::Retorno);
    assert_eq!(pc(&d), 3, "tras el CALL externa, no tras el CALL interna");
}

#[test]
fn salir_desde_la_subrutina_interna_vuelve_a_la_externa() {
    let mut d = depurador();
    d.paso(3); // CALL externa; NOP; CALL interna
    assert_eq!(d.salir(100).unwrap(), Parada::Retorno);
    // De vuelta en `externa`, justo despues de `CALL interna`.
    assert!(pc(&d) > 3);
    assert_eq!(d.salir(100).unwrap(), Parada::Retorno);
    assert_eq!(pc(&d), 3);
}

#[test]
fn salir_respeta_el_limite_de_instrucciones() {
    let mut d = depurador();
    d.paso(1);
    assert!(matches!(d.salir(1).unwrap(), Parada::Limite { .. }));
}
