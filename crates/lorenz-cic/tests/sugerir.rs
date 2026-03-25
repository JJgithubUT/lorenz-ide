//! Sugerencias de instrucciones nuevas (RF-CIC-034).

use lorenz_cic::sugerir::{sugerir, trazar, Opciones};
use lorenz_emu::{Emulador, Paso};

fn paso(direccion: u64, mnemonico: &str, longitud: usize, ciclos: u64) -> Paso {
    Paso {
        direccion,
        mnemonico: mnemonico.into(),
        longitud,
        ciclos,
        salto: false,
        detenida: false,
    }
}

/// `LDI` (2 palabras, 2 ciclos) seguido de `ADD` (1 palabra, 1 ciclo), repetido `n` veces en
/// direcciones distintas.
fn pares(n: u64) -> Vec<Paso> {
    let mut v = Vec::new();
    for k in 0..n {
        v.push(paso(k * 3, "LDI", 2, 2));
        v.push(paso(k * 3 + 2, "ADD", 1, 1));
    }
    v
}

#[test]
fn una_pareja_repetida_se_sugiere_con_su_ahorro() {
    let s = sugerir(&[pares(3)], &Opciones::default());
    let p = s.iter().find(|s| s.nombre() == "LDI+ADD").unwrap();
    assert_eq!(p.apariciones, 3);
    assert_eq!(p.ejecuciones, 3);
    // Cada pareja ocupa 3 palabras y tardaria lo que la mas larga (2): se ahorra 1 palabra y 1 ciclo.
    assert_eq!(p.palabras_ahorradas, 3);
    assert_eq!(p.ciclos_ahorrados, 3);
}

#[test]
fn lo_que_mas_ahorra_va_primero() {
    let mut v = pares(2);
    // Una pareja que se ejecuta cien veces en un bucle.
    for _ in 0..100 {
        v.push(paso(100, "LD", 3, 4));
        v.push(paso(103, "ADD", 1, 1));
        v.push(paso(104, "JZ", 3, 2));
    }
    let s = sugerir(&[v], &Opciones::default());
    assert_eq!(s[0].patron[0], "LD");
    assert!(s[0].ciclos_ahorrados >= 100);
}

#[test]
fn una_secuencia_no_cruza_un_destino_de_salto() {
    // A B C D, pero se salta a C desde otro sitio: B+C no puede ser una sola instruccion.
    let v = vec![
        paso(0, "A", 1, 1),
        paso(1, "B", 1, 1),
        paso(2, "C", 1, 1),
        paso(3, "D", 1, 1),
        paso(0, "A", 1, 1),
        paso(2, "C", 1, 1),
        paso(3, "D", 1, 1),
    ];
    let s = sugerir(&[v], &Opciones::default());
    assert!(!s.iter().any(|s| s.nombre() == "B+C"), "{s:?}");
    assert!(!s.iter().any(|s| s.nombre() == "A+B+C"), "{s:?}");
    assert!(s.iter().any(|s| s.nombre() == "C+D"));
}

#[test]
fn una_instruccion_que_salta_cierra_la_secuencia() {
    let v = vec![
        paso(0, "CMP", 1, 1),
        paso(1, "JZ", 2, 2),
        paso(10, "NOP", 1, 1),
        paso(0, "CMP", 1, 1),
        paso(1, "JZ", 2, 2),
        paso(10, "NOP", 1, 1),
    ];
    let s = sugerir(&[v], &Opciones::default());
    assert!(s.iter().any(|s| s.nombre() == "CMP+JZ"));
    assert!(
        !s.iter().any(|s| s.nombre().starts_with("CMP+JZ+")),
        "{s:?}"
    );
}

#[test]
fn la_longitud_y_la_cantidad_se_acotan() {
    let o = Opciones {
        minima: 3,
        maxima: 3,
        cuantas: 1,
    };
    let v = vec![
        paso(0, "A", 1, 1),
        paso(1, "B", 1, 1),
        paso(2, "C", 1, 1),
        paso(3, "D", 1, 1),
    ];
    let s = sugerir(&[v], &o);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].patron.len(), 3);
}

#[test]
fn varios_programas_suman_sus_apariciones() {
    let s = sugerir(&[pares(2), pares(3)], &Opciones::default());
    assert_eq!(
        s.iter()
            .find(|s| s.nombre() == "LDI+ADD")
            .unwrap()
            .apariciones,
        5
    );
}

#[test]
fn sin_nada_que_ahorrar_no_se_sugiere_nada() {
    // Una sola instruccion no forma ninguna secuencia.
    let v = vec![paso(0, "A", 1, 1)];
    assert!(sugerir(&[v], &Opciones::default()).is_empty());
    assert!(sugerir(&[], &Opciones::default()).is_empty());
}

#[test]
fn sobre_un_programa_real_encuentra_las_parejas_del_bucle() {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let mut e = Emulador::nuevo(m).unwrap();
    // ADD A,B ; JZ inicio (bucle infinito)
    e.cargar_programa(0, &[0x31, 0x60, 0x00, 0x00]).unwrap();
    let traza = trazar(&mut e, 50).unwrap();
    assert_eq!(traza.len(), 50);
    let s = sugerir(&[traza], &Opciones::default());
    assert_eq!(s[0].nombre(), "ADD+JZ");
    assert!(s[0].ejecuciones >= 25);
}
