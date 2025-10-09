//! Generacion de casos de conformidad (RF-TST-016, RF-TST-020).

use std::path::PathBuf;

use lorenz_arch::modelo::{Maquina, TipoOperando};
use lorenz_tst::casos::{
    candidatos, casos_de, combinaciones, fuera_de_rango, patrones, region_de_pruebas,
};

fn maquina() -> Maquina {
    let ruta =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ejemplos/ejemplo-8/maquina.toml");
    lorenz_arch::carga::cargar_archivo(ruta).unwrap()
}

#[test]
fn hay_una_region_de_ram_para_poner_las_instrucciones() {
    let m = maquina();
    let r = region_de_pruebas(&m).expect("el ejemplo tiene RAM");
    assert!(r.acceso.escritura);
}

#[test]
fn los_inmediatos_incluyen_los_extremos_del_rango() {
    let m = maquina();
    let ldi = m.instruccion("LDI").unwrap();
    let imm = ldi.operandos.iter().find(|o| o.nombre == "imm").unwrap();
    let TipoOperando::Inmediato { rango, .. } = &imm.tipo else {
        panic!("imm no es inmediato");
    };
    let c = candidatos(&m, ldi, imm);
    assert!(c.contains(&i128::from(rango.0)));
    assert!(c.contains(&i128::from(rango.1)));
    assert!(c.contains(&0) && c.contains(&1));
    assert!(c
        .iter()
        .all(|v| *v >= i128::from(rango.0) && *v <= i128::from(rango.1)));
}

#[test]
fn los_registros_se_prueban_todos() {
    let m = maquina();
    let mov = m.instruccion("MOV").unwrap();
    let rd = mov.operandos.iter().find(|o| o.nombre == "rd").unwrap();
    assert_eq!(candidatos(&m, mov, rd), vec![0, 1, 2, 3]);
    assert_eq!(combinaciones(&m, mov, 1000).len(), 16);
}

#[test]
fn con_un_tope_se_reparten_por_todo_el_espacio() {
    let m = maquina();
    let mov = m.instruccion("MOV").unwrap();
    let c = combinaciones(&m, mov, 5);
    assert_eq!(c.len(), 5);
    assert_eq!(c[0], vec![("rd".to_string(), 0), ("rs".to_string(), 0)]);
    assert_eq!(c[4], vec![("rd".to_string(), 3), ("rs".to_string(), 3)]);
}

#[test]
fn fuera_de_rango_da_los_vecinos_de_los_extremos() {
    let m = maquina();
    let ldi = m.instruccion("LDI").unwrap();
    let imm = ldi.operandos.iter().find(|o| o.nombre == "imm").unwrap();
    let TipoOperando::Inmediato { rango, .. } = &imm.tipo else {
        unreachable!()
    };
    assert_eq!(
        fuera_de_rango(imm),
        vec![i128::from(rango.0) - 1, i128::from(rango.1) + 1]
    );
}

#[test]
fn los_patrones_son_deterministas_y_variados() {
    let m = maquina();
    let a = patrones(&m, 8);
    assert_eq!(a, patrones(&m, 8));
    assert_eq!(a.len(), 8);
    assert!(a.windows(2).any(|w| w[0] != w[1]));
    // Los dos valores de las banderas aparecen.
    assert!(a.iter().any(|p| p.banderas.iter().any(|(_, v)| *v)));
    assert!(a.iter().any(|p| p.banderas.iter().any(|(_, v)| !*v)));
}

#[test]
fn los_casos_de_una_instruccion_repiten_cada_combinacion_con_estados_distintos() {
    let m = maquina();
    let idx = m
        .instrucciones
        .iter()
        .position(|i| i.mnemonico == "ADD")
        .unwrap();
    let casos = casos_de(&m, idx, 100, 3, &patrones(&m, 8));
    assert_eq!(casos.len(), 16 * 3);
    assert_ne!(casos[0].patron, casos[1].patron);
    assert!(casos[0].describir(&m).starts_with("ADD ["));
}
