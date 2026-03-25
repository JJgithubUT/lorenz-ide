//! Anotacion del trafico de datos de cada instruccion, base del modelo de cache (RF-CIC-018).

mod comun;

use comun::{con_tres_regiones, ejemplo_8};
use lorenz_emu::{Emulador, Memoria, Trafico};

#[test]
fn sin_activarlo_no_se_anota_nada() {
    let mut m = Memoria::nueva(&con_tres_regiones()).unwrap();
    m.escribir(0x100, 1).unwrap();
    assert!(m.tomar_trafico().is_empty());
}

#[test]
fn se_anotan_lecturas_y_escrituras_en_orden_y_se_vacian_al_tomarlas() {
    let mut m = Memoria::nueva(&con_tres_regiones()).unwrap();
    m.anotar_trafico(true);
    m.escribir(0x100, 1).unwrap();
    m.leer_vigilada(0x100).unwrap();
    assert_eq!(
        m.tomar_trafico(),
        vec![
            Trafico {
                direccion: 0x100,
                escritura: true
            },
            Trafico {
                direccion: 0x100,
                escritura: false
            }
        ]
    );
    assert!(m.tomar_trafico().is_empty());
}

#[test]
fn una_carga_y_un_almacenamiento_dejan_su_rastro() {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    // LDI A,0x42 ; ST A,[0x2000] ; LD B,[0x2000] ; HLT
    let programa = [0x20, 0x42, 0x50, 0x00, 0x20, 0x44, 0x00, 0x20, 0xF0];
    e.cargar_programa(0, &programa).unwrap();
    e.memoria_mut().anotar_trafico(true);
    let mut rastros = Vec::new();
    while e.paso().unwrap().is_some() {
        rastros.push(e.memoria_mut().tomar_trafico());
    }
    assert!(rastros[0].is_empty(), "{rastros:?}");
    assert_eq!(
        rastros[1],
        vec![Trafico {
            direccion: 0x2000,
            escritura: true
        }]
    );
    assert_eq!(
        rastros[2],
        vec![Trafico {
            direccion: 0x2000,
            escritura: false
        }]
    );
}
