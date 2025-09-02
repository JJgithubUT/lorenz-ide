//! Pruebas de la seccion de interrupciones de la descripcion: vectores y prioridades.

use lorenz_arch::{cargar, ErrorArquitectura, Regla};

fn ejemplo() -> String {
    std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap()
}

fn con_prioridades(linea: &str) -> String {
    ejemplo().replace(
        "habilitadas_al_arrancar = false",
        &format!("habilitadas_al_arrancar = false\n{linea}"),
    )
}

fn reglas(texto: &str) -> Vec<Regla> {
    match cargar(texto) {
        Ok(_) => Vec::new(),
        Err(ErrorArquitectura::Validacion(f)) => f.into_iter().map(|x| x.regla).collect(),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn sin_prioridades_cada_linea_usa_su_propio_numero() {
    let m = cargar(&ejemplo()).unwrap();
    let i = m.interrupciones.unwrap();
    assert!(i.prioridades.is_empty());
    assert_eq!(i.prioridad_de(0), 0);
    assert_eq!(i.prioridad_de(1), 1);
}

#[test]
fn las_prioridades_declaradas_se_cargan() {
    let m = cargar(&con_prioridades("prioridades = { 0 = 5, 1 = 2 }")).unwrap();
    let i = m.interrupciones.unwrap();
    assert_eq!(i.prioridad_de(0), 5);
    assert_eq!(i.prioridad_de(1), 2);
}

#[test]
fn una_linea_sin_prioridad_usa_su_numero() {
    let m = cargar(&con_prioridades("prioridades = { 1 = 9 }")).unwrap();
    let i = m.interrupciones.unwrap();
    assert_eq!(i.prioridad_de(0), 0);
    assert_eq!(i.prioridad_de(1), 9);
}

#[test]
fn una_prioridad_para_una_linea_sin_vector_se_rechaza() {
    let r = reglas(&con_prioridades("prioridades = { 7 = 1 }"));
    assert_eq!(r, [Regla::ValorFueraDeRango]);
}

#[test]
fn una_linea_sin_vector_tiene_la_prioridad_de_su_numero_en_cualquier_caso() {
    let m = cargar(&ejemplo()).unwrap();
    assert_eq!(m.interrupciones.unwrap().prioridad_de(40), 40);
}
