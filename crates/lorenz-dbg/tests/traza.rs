//! La estructura de la traza, sin el depurador (RF-DEP-019, RF-DEP-027, RF-DEP-028).

use lorenz_dbg::traza::{CambioRegistro, Intervencion, PasoTraza, Traza};
use lorenz_emu::{Emulador, EscrituraRegistrada};

fn emu() -> Emulador {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    Emulador::nuevo(lorenz_arch::cargar(&t).unwrap()).unwrap()
}

fn paso(instante: u64) -> PasoTraza {
    PasoTraza {
        instante,
        pc: instante,
        mnemonico: "NOP".into(),
        ciclos: 1,
        registros: vec![],
        banderas: vec![],
        escrituras: vec![],
    }
}

fn traza_de(n: u64) -> Traza {
    let e = emu();
    let mut t = Traza::nueva(e.instantanea());
    for i in 0..n {
        t.anotar(paso(i), || e.instantanea());
    }
    t
}

#[test]
fn empieza_vacia_con_la_instantanea_inicial() {
    let t = Traza::nueva(emu().instantanea());
    assert_eq!(t.longitud(), 0);
    assert_eq!(t.instantaneas(), 1);
    assert!(!t.llena());
}

#[test]
fn anota_pasos_en_orden() {
    let t = traza_de(5);
    assert_eq!(t.longitud(), 5);
    assert_eq!(t.pasos()[3].instante, 3);
}

#[test]
fn toma_una_instantanea_cada_intervalo() {
    let e = emu();
    let mut t = Traza::nueva(e.instantanea());
    t.fijar_intervalo(10);
    for i in 0..35 {
        t.anotar(paso(i), || e.instantanea());
    }
    assert_eq!(t.instantaneas(), 4); // 0, 10, 20, 30
    assert_eq!(t.instantanea_hasta(25).0, 20);
    assert_eq!(t.instantanea_hasta(30).0, 30);
    assert_eq!(t.instantanea_hasta(0).0, 0);
}

#[test]
fn la_memoria_de_instantaneas_tiene_tope() {
    let e = emu();
    let mut t = Traza::nueva(e.instantanea());
    t.fijar_intervalo(1);
    for i in 0..1000 {
        t.anotar(paso(i), || e.instantanea());
    }
    assert!(t.instantaneas() <= lorenz_dbg::traza::MAX_INSTANTANEAS);
    assert!(t.intervalo() > 1);
    // El cero nunca se pierde.
    assert_eq!(t.instantanea_hasta(0).0, 0);
}

#[test]
fn el_limite_detiene_la_grabacion_y_avisa() {
    let e = emu();
    let mut t = Traza::nueva(e.instantanea());
    t.fijar_limite(3);
    assert!(t.anotar(paso(0), || e.instantanea()));
    assert!(t.anotar(paso(1), || e.instantanea()));
    assert!(!t.llena());
    assert!(t.anotar(paso(2), || e.instantanea()));
    assert!(t.llena());
    assert!(!t.anotar(paso(3), || e.instantanea()));
    assert_eq!(t.longitud(), 3);
}

#[test]
fn truncar_reescribe_la_historia() {
    let mut t = traza_de(10);
    t.intervenir(
        2,
        Intervencion::Memoria {
            direccion: 1,
            valor: 1,
        },
    );
    t.intervenir(
        7,
        Intervencion::Memoria {
            direccion: 2,
            valor: 2,
        },
    );
    t.truncar(5);
    assert_eq!(t.longitud(), 5);
    assert_eq!(t.intervenciones().len(), 1);
    t.truncar(0);
    assert!(t.intervenciones().is_empty());
}

#[test]
fn truncar_libera_el_limite() {
    let e = emu();
    let mut t = Traza::nueva(e.instantanea());
    t.fijar_limite(2);
    t.anotar(paso(0), || e.instantanea());
    t.anotar(paso(1), || e.instantanea());
    assert!(t.llena());
    t.truncar(1);
    assert!(!t.llena());
}

#[test]
fn intervenciones_en_un_instante() {
    let mut t = traza_de(3);
    t.intervenir(
        1,
        Intervencion::Registro {
            nombre: "A".into(),
            valor: 9,
        },
    );
    t.intervenir(
        1,
        Intervencion::Entrada {
            numero: 1,
            valor: 5,
        },
    );
    t.intervenir(
        2,
        Intervencion::Puerto {
            numero: 1,
            valor: 5,
        },
    );
    assert_eq!(t.intervenciones_en(1).count(), 2);
    assert_eq!(t.intervenciones_en(0).count(), 0);
}

#[test]
fn ultima_escritura_de_una_direccion() {
    let e = emu();
    let mut t = Traza::nueva(e.instantanea());
    for i in 0..6 {
        let mut p = paso(i);
        if i == 1 || i == 4 {
            p.escrituras.push(EscrituraRegistrada {
                direccion: 0x2000,
                antes: 0,
                despues: i,
                pc: i,
            });
        }
        t.anotar(p, || e.instantanea());
    }
    assert_eq!(t.ultima_escritura(0x2000, 6).unwrap().instante, 4);
    assert_eq!(t.ultima_escritura(0x2000, 4).unwrap().instante, 1);
    assert!(t.ultima_escritura(0x2000, 1).is_none());
    assert!(t.ultima_escritura(0x2001, 6).is_none());
}

#[test]
fn ultimo_cambio_de_registro() {
    let e = emu();
    let mut t = Traza::nueva(e.instantanea());
    for i in 0..4 {
        let mut p = paso(i);
        if i == 2 {
            p.registros.push(CambioRegistro {
                nombre: "A".into(),
                antes: 0,
                despues: 1,
            });
        }
        t.anotar(p, || e.instantanea());
    }
    assert_eq!(t.ultimo_cambio_de_registro("a", 4).unwrap().instante, 2);
    assert!(t.ultimo_cambio_de_registro("B", 4).is_none());
}
