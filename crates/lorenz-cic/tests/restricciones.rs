use lorenz_cic::{comprobar, Medidas, Metrica, Metricas, Restriccion};

fn medidas(ciclos: u64, instrucciones: u64, palabras: u64) -> Medidas {
    Medidas::new(
        Metricas {
            ciclos,
            instrucciones,
            ..Metricas::default()
        },
        palabras,
    )
}

fn r(m: Metrica, min: Option<f64>, max: Option<f64>) -> Restriccion {
    Restriccion {
        nombre: m.nombre().into(),
        metrica: m,
        minimo: min,
        maximo: max,
    }
}

#[test]
fn un_maximo_se_cumple_en_el_limite_y_por_debajo() {
    let m = medidas(100, 40, 10);
    let v = comprobar(&[r(Metrica::Ciclos, None, Some(100.0))], &m);
    assert!(v[0].cumple);
    assert_eq!(v[0].valor, Some(100.0));
    let v = comprobar(&[r(Metrica::Ciclos, None, Some(99.0))], &m);
    assert!(!v[0].cumple);
}

#[test]
fn un_minimo_se_comprueba_por_arriba() {
    let m = medidas(100, 40, 10);
    assert!(comprobar(&[r(Metrica::Instrucciones, Some(40.0), None)], &m)[0].cumple);
    assert!(!comprobar(&[r(Metrica::Instrucciones, Some(41.0), None)], &m)[0].cumple);
}

#[test]
fn el_cpi_es_ciclos_entre_instrucciones() {
    let m = medidas(100, 40, 10);
    let v = comprobar(&[r(Metrica::Cpi, None, Some(2.5))], &m);
    assert_eq!(v[0].valor, Some(2.5));
    assert!(v[0].cumple);
}

#[test]
fn las_palabras_de_codigo_vienen_de_fuera_del_simulador() {
    let m = medidas(100, 40, 10);
    assert!(comprobar(&[r(Metrica::PalabrasCodigo, None, Some(10.0))], &m)[0].cumple);
    assert!(!comprobar(&[r(Metrica::PalabrasCodigo, None, Some(9.0))], &m)[0].cumple);
}

#[test]
fn una_cifra_que_no_se_puede_medir_no_cumple() {
    let m = medidas(0, 0, 0);
    let v = comprobar(&[r(Metrica::Cpi, None, Some(5.0))], &m);
    assert_eq!(v[0].valor, None);
    assert!(!v[0].cumple);
    // sin cache no hay fallos de cache que contar
    let v = comprobar(
        &[r(Metrica::FallosCache, None, Some(5.0))],
        &medidas(10, 5, 5),
    );
    assert!(!v[0].cumple);
}

#[test]
fn los_nombres_de_las_metricas_se_leen_y_se_escriben() {
    for m in Metrica::TODAS {
        assert_eq!(Metrica::desde_nombre(m.nombre()), Ok(m));
    }
    let e = Metrica::desde_nombre("velocidad").unwrap_err();
    assert!(e.contains("ciclos") && e.contains("velocidad"));
}

#[test]
fn una_restriccion_sin_limites_o_contradictoria_no_vale() {
    assert!(r(Metrica::Ciclos, None, None).validar().is_err());
    assert!(r(Metrica::Ciclos, Some(5.0), Some(2.0)).validar().is_err());
    assert!(r(Metrica::Ciclos, Some(2.0), Some(5.0)).validar().is_ok());
}
