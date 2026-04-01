mod comun;

use comun::*;
use lorenz_cic::{
    cargar_config, comprobar, evaluar, variantes, variantes_isa, Espacio, Medidas, Metrica,
    Metricas, Prediccion, Restriccion,
};
use lorenz_emu::Emulador;

/// ADD A,B deja Z encendida (0 + 0); JZ 5 salta siempre; el NOP del medio no se ejecuta.
const SALTA: [u8; 6] = [ADD_AB, 0x60, 0x05, 0x00, 0x00, HLT];

fn config(extra: &str) -> lorenz_cic::Config {
    cargar_config(&format!("{}\n{extra}\n", texto_maquina())).unwrap()
}

fn emulador() -> Result<Emulador, String> {
    let m = lorenz_arch::cargar(&texto_maquina()).map_err(|e| e.to_string())?;
    let mut e = Emulador::nuevo(m).map_err(|e| e.to_string())?;
    let palabras: Vec<u64> = SALTA.iter().map(|b| u64::from(*b)).collect();
    e.cargar_programa(0, &palabras).map_err(|e| e.to_string())?;
    Ok(e)
}

fn max_ciclos(n: f64) -> Restriccion {
    Restriccion {
        nombre: "ciclos".into(),
        metrica: Metrica::Ciclos,
        minimo: None,
        maximo: Some(n),
    }
}

#[test]
fn sin_pipeline_ni_cache_solo_existe_la_base() {
    let v = variantes(&config(""), &Espacio::default());
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].nombre, "base");
}

#[test]
fn con_pipeline_se_varian_adelantamiento_y_prediccion() {
    let v = variantes(&config(CINCO), &Espacio::default());
    // 2 adelantamientos x 4 predicciones, y una de ellas es la base
    assert_eq!(v.len(), 8);
    assert_eq!(v[0].nombre, "base");
    assert!(v.iter().any(|x| x.nombre.contains("prediccion=2-bits")));
    assert!(v.iter().any(|x| x.nombre.contains("adelantamiento=no")));
    let sin_repetidas = v.iter().enumerate().all(|(i, a)| {
        v.iter()
            .enumerate()
            .all(|(j, b)| i == j || a.config != b.config)
    });
    assert!(sin_repetidas);
}

#[test]
fn los_factores_de_la_cache_dan_tamanos_validos_y_no_repetidos() {
    let cfg = config(&format!(
        "{CINCO}[cache]\ntamano = 16\nlinea = 2\nvias = 2\nlatencia_acierto = 1\n"
    ));
    let esp = Espacio {
        adelantamiento: false,
        predicciones: vec![],
        factores_cache: vec![0.5, 1.0, 2.0],
        bus: false,
    };
    let v = variantes(&cfg, &esp);
    let tamanos: Vec<u64> = v
        .iter()
        .map(|x| x.config.cache.as_ref().unwrap().tamano)
        .collect();
    assert_eq!(tamanos, vec![16, 8, 32], "{v:?}");
}

#[test]
fn un_tamano_de_cache_que_no_cuadra_se_salta() {
    let cfg = config(&format!(
        "{CINCO}[cache]\ntamano = 16\nlinea = 2\nvias = 2\nlatencia_acierto = 1\n"
    ));
    let esp = Espacio {
        adelantamiento: false,
        predicciones: vec![],
        factores_cache: vec![0.01, 2.0],
        bus: false,
    };
    let v = variantes(&cfg, &esp);
    assert_eq!(v.len(), 2, "{v:?}");
}

#[test]
fn el_bus_compartido_es_otra_dimension() {
    let esp = Espacio {
        adelantamiento: false,
        predicciones: vec![],
        factores_cache: vec![],
        bus: true,
    };
    let v = variantes(&config(CINCO), &esp);
    assert_eq!(v.len(), 2);
    assert!(v[1].nombre.contains("bus=compartido"));
}

#[test]
fn evaluar_pone_primero_las_que_cumplen_y_las_mas_rapidas() {
    let cfg = config(CINCO);
    let vs = variantes(&cfg, &Espacio::default());
    let todas = evaluar(&vs, &[], 6, 100, &mut emulador).unwrap();
    let ciclos: Vec<u64> = todas.iter().map(|r| r.medidas.metricas.ciclos).collect();
    let mut ordenados = ciclos.clone();
    ordenados.sort();
    assert_eq!(ciclos, ordenados);
    assert!(
        todas.iter().all(|r| r.cumple),
        "sin restricciones todo cumple"
    );
    let (mejor, peor) = (ciclos[0], *ciclos.last().unwrap());
    assert!(
        mejor < peor,
        "las variantes deben dar cifras distintas: {ciclos:?}"
    );
    // un limite entre las dos deja fuera a las lentas
    let limite = (mejor + peor) / 2;
    let r = evaluar(&vs, &[max_ciclos(limite as f64)], 6, 100, &mut emulador).unwrap();
    let cumplen = r.iter().filter(|x| x.cumple).count();
    assert!(cumplen >= 1 && cumplen < r.len());
    assert!(r[..cumplen].iter().all(|x| x.cumple));
    assert!(r[cumplen..].iter().all(|x| !x.cumple));
    assert_eq!(r[0].medidas.metricas.ciclos, mejor);
}

#[test]
fn predecir_que_se_salta_mejora_un_programa_que_siempre_salta() {
    let vs = variantes(&config(CINCO), &Espacio::default());
    let r = evaluar(&vs, &[], 6, 100, &mut emulador).unwrap();
    let ciclos = |n: &str| {
        r.iter()
            .find(|x| x.nombre.contains(n) && x.nombre.contains("adelantamiento=si"))
            .unwrap()
            .medidas
            .metricas
            .ciclos
    };
    // la combinacion no-tomado con adelantamiento es la propia base, que no se repite
    let base = r
        .iter()
        .find(|x| x.nombre == "base")
        .unwrap()
        .medidas
        .metricas
        .ciclos;
    assert!(ciclos("prediccion=tomado") < base);
}

#[test]
fn si_el_programa_no_termina_el_error_nombra_la_variante() {
    let vs = variantes(&config(CINCO), &Espacio::default());
    let e = evaluar(&vs, &[], 6, 1, &mut emulador).unwrap_err();
    assert!(e.contains("base") && e.contains("no termino"), "{e}");
}

fn medidas(ciclos: u64, instr: u64, palabras: u64) -> Medidas {
    Medidas::new(
        Metricas {
            ciclos,
            instrucciones: instr,
            ..Metricas::default()
        },
        palabras,
    )
}

#[test]
fn las_variantes_de_isa_restan_lo_que_la_sugerencia_ahorra() {
    use lorenz_cic::sugerir::Sugerencia;
    let s = Sugerencia {
        patron: vec!["LDI".into(), "ADD".into()],
        apariciones: 2,
        ejecuciones: 10,
        palabras_ahorradas: 3,
        ciclos_ahorrados: 20,
    };
    let base = medidas(100, 50, 12);
    let r = variantes_isa(&base, &[s], &[max_ciclos(90.0)]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].instruccion, "LDI+ADD");
    assert_eq!(r[0].medidas.metricas.ciclos, 80);
    assert_eq!(r[0].medidas.palabras_codigo, 9);
    assert_eq!(r[0].medidas.metricas.instrucciones, 40);
    assert!(r[0].cumple);
    // la base sola no cumplia
    assert!(!comprobar(&[max_ciclos(90.0)], &base)[0].cumple);
}

#[test]
fn las_variantes_de_isa_ponen_primero_las_que_cumplen() {
    use lorenz_cic::sugerir::Sugerencia;
    let mk = |n: &str, c| Sugerencia {
        patron: vec![n.into(), "X".into()],
        apariciones: 1,
        ejecuciones: 1,
        palabras_ahorradas: 0,
        ciclos_ahorrados: c,
    };
    let r = variantes_isa(
        &medidas(100, 10, 5),
        &[mk("A", 2), mk("B", 30)],
        &[max_ciclos(80.0)],
    );
    assert_eq!(r[0].instruccion, "B+X");
    assert!(r[0].cumple && !r[1].cumple);
}

#[test]
fn prediccion_nombre_ida_y_vuelta_para_las_cuatro() {
    for p in [
        Prediccion::NoTomado,
        Prediccion::Tomado,
        Prediccion::UnBit,
        Prediccion::DosBits,
    ] {
        assert_eq!(Prediccion::desde_nombre(p.nombre()), Ok(p));
    }
}
