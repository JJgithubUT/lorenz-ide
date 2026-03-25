//! Exploracion de ordenes de eventos para encontrar condiciones de carrera (RF-DEP-043).

use lorenz_dbg::explorar::{explorar, Opciones};
use lorenz_dbg::traza::Intervencion;
use lorenz_dbg::{Depurador, ErrorDepurador};
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

/// El programa principal suma 1 a un contador leyendo y guardando; el manejador de la linea 0
/// tambien. Si la interrupcion cae entre la lectura y el guardado, una suma se pierde.
const CON_CARRERA: &str = "\
        EI
        LD  A, [0x2000]
        LDI B, 1
        ADD A, B
        ST  [0x2000], A
        HLT
        .org 0x0100
        LD  C, [0x2000]
        LDI D, 1
        ADD C, D
        ST  [0x2000], C
        RETI
        .org 0xEFF0
        .db 0x00, 0x01
";

/// Igual, pero el programa principal inhabilita las interrupciones mientras actualiza.
const SIN_CARRERA: &str = "\
        DI
        LD  A, [0x2000]
        LDI B, 1
        ADD A, B
        ST  [0x2000], A
        EI
        NOP
        NOP
        HLT
        .org 0x0100
        LD  C, [0x2000]
        LDI D, 1
        ADD C, D
        ST  [0x2000], C
        RETI
        .org 0xEFF0
        .db 0x00, 0x01
";

fn depurador(fuente: &str) -> Depurador {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", fuente))
        .unwrap();
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    Depurador::nuevo(emu)
}

fn interrupcion() -> Vec<Intervencion> {
    vec![Intervencion::Interrupcion { linea: 0 }]
}

fn opciones() -> Opciones {
    Opciones {
        ventana: 12,
        celdas: vec![0x2000],
        ..Opciones::default()
    }
}

#[test]
fn una_interrupcion_entre_leer_y_guardar_da_dos_resultados() {
    let d = depurador(CON_CARRERA);
    let r = explorar(&d, &interrupcion(), &opciones()).unwrap();
    assert!(r.hay_carrera(), "{r:?}");
    assert!(r.completa);
    assert_eq!(r.casos, 13);
    let celdas: Vec<&str> = r
        .grupos
        .iter()
        .map(|g| {
            if g.resultado.contains("[0x2000]=0x2 ") {
                "2"
            } else if g.resultado.contains("[0x2000]=0x1 ") {
                "1"
            } else {
                "otro"
            }
        })
        .collect();
    assert!(celdas.contains(&"1") && celdas.contains(&"2"), "{celdas:?}");
}

#[test]
fn cada_grupo_trae_un_orden_que_lo_reproduce() {
    let d = depurador(CON_CARRERA);
    let r = explorar(&d, &interrupcion(), &opciones()).unwrap();
    for g in &r.grupos {
        let (instante, _) = g.ejemplo[0];
        let mut copia = depurador(CON_CARRERA);
        copia
            .programar_inyeccion(
                instante,
                lorenz_dbg::inyeccion::Inyeccion::Interrupcion { linea: 0 },
            )
            .unwrap();
        copia.continuar(100_000);
        let v = copia
            .emulador()
            .memoria()
            .leer_privilegiado(0x2000)
            .unwrap();
        assert!(
            g.resultado.contains(&format!("[0x2000]={v:#x} ")),
            "{} {v}",
            g.resultado
        );
    }
}

#[test]
fn inhabilitar_las_interrupciones_mientras_se_actualiza_quita_la_carrera() {
    let d = depurador(SIN_CARRERA);
    let r = explorar(&d, &interrupcion(), &opciones()).unwrap();
    let con_dos: u64 = r
        .grupos
        .iter()
        .filter(|g| g.resultado.contains("[0x2000]=0x2 "))
        .map(|g| g.casos)
        .sum();
    let con_uno: u64 = r
        .grupos
        .iter()
        .filter(|g| g.resultado.contains("[0x2000]=0x1 "))
        .map(|g| g.casos)
        .sum();
    // Solo se pierde la interrupcion que llega cuando la maquina ya se habia detenido.
    assert!(con_dos > con_uno, "{r:?}");
    assert!(r
        .grupos
        .iter()
        .all(|g| !g.resultado.contains("[0x2000]=0x0 ")));
}

#[test]
fn explorar_no_cambia_la_maquina_que_se_depura() {
    let mut d = depurador(CON_CARRERA);
    d.paso(2);
    let antes = format!("{:?}", d.emulador().estado());
    explorar(&d, &interrupcion(), &opciones()).unwrap();
    assert_eq!(format!("{:?}", d.emulador().estado()), antes);
}

#[test]
fn la_exploracion_es_repetible() {
    let d = depurador(CON_CARRERA);
    let a = explorar(&d, &interrupcion(), &opciones()).unwrap();
    let b = explorar(&d, &interrupcion(), &opciones()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn con_dos_eventos_se_prueban_tambien_los_empates_en_los_dos_ordenes() {
    let d = depurador(CON_CARRERA);
    let eventos = vec![
        Intervencion::Registro {
            nombre: "D".into(),
            valor: 1,
        },
        Intervencion::Registro {
            nombre: "D".into(),
            valor: 2,
        },
    ];
    let o = Opciones {
        ventana: 0,
        ..opciones()
    };
    let r = explorar(&d, &eventos, &o).unwrap();
    // Los dos a la vez: dos ordenes posibles y dos valores finales de D.
    assert_eq!(r.casos, 2);
    assert!(r.hay_carrera());
}

#[test]
fn si_hay_demasiados_casos_se_toma_una_muestra() {
    let d = depurador(CON_CARRERA);
    let o = Opciones {
        ventana: 50,
        max_casos: 20,
        ..opciones()
    };
    let eventos = vec![
        Intervencion::Interrupcion { linea: 0 },
        Intervencion::Interrupcion { linea: 1 },
    ];
    let r = explorar(&d, &eventos, &o).unwrap();
    assert!(!r.completa);
    assert!(r.casos <= 20);
}

#[test]
fn sin_eventos_o_con_demasiados_se_rechaza() {
    let d = depurador(CON_CARRERA);
    assert!(matches!(
        explorar(&d, &[], &opciones()),
        Err(ErrorDepurador::FaltaArgumento { .. })
    ));
    let muchos = vec![Intervencion::Interrupcion { linea: 0 }; 6];
    assert!(matches!(
        explorar(&d, &muchos, &opciones()),
        Err(ErrorDepurador::DemasiadosEventos { .. })
    ));
}
