//! Comparar dos ejecuciones de la suite (RF-TST-027, RF-TST-028, RF-TST-036).

use lorenz_tst::ejecutor::{Fallo, Resultado, Veredicto};
use lorenz_tst::regresion::{comparar_resultados, Cambio, Umbral};
use lorenz_tst::suite::{Informe, Resumen, FORMATO_RESULTADOS};

fn prueba(nombre: &str, v: Veredicto, ciclos: u64) -> Resultado {
    Resultado {
        nombre: nombre.into(),
        etiquetas: vec![],
        veredicto: v,
        fallos: if v == Veredicto::Falla {
            vec![Fallo {
                que: "registro A".into(),
                esperado: "0x01".into(),
                obtenido: "0x02".into(),
                donde: None,
            }]
        } else {
            vec![]
        },
        motivo: None,
        instrucciones: ciclos,
        ciclos,
        milisegundos: 1,
    }
}

fn informe(pruebas: Vec<Resultado>) -> Informe {
    Informe {
        formato: FORMATO_RESULTADOS.into(),
        proyecto: "p.toml".into(),
        maquina: "m".into(),
        semilla: 0,
        resumen: Resumen {
            total: pruebas.len(),
            pasan: 0,
            fallan: 0,
            errores: 0,
            omitidas: 0,
        },
        milisegundos: 1,
        pruebas,
        cobertura: None,
        regresion: None,
    }
}

#[test]
fn dos_ejecuciones_iguales_no_tienen_regresion() {
    let a = informe(vec![
        prueba("a", Veredicto::Pasa, 10),
        prueba("b", Veredicto::Pasa, 20),
    ]);
    let r = comparar_resultados(&a, &a.clone(), Umbral::default());
    assert!(r.cambios.is_empty());
    assert_eq!(r.iguales, 2);
    assert!(!r.hay_regresion());
    assert_eq!(r.codigo_de_salida(), 0);
    assert!(r.a_texto().contains("2 sin cambios; sin regresion"));
}

#[test]
fn una_prueba_que_pasaba_y_ahora_falla_se_rompe() {
    let antes = informe(vec![prueba("suma", Veredicto::Pasa, 10)]);
    let ahora = informe(vec![prueba("suma", Veredicto::Falla, 10)]);
    let r = comparar_resultados(&antes, &ahora, Umbral::default());
    assert!(
        matches!(&r.cambios[0], Cambio::Rompe { nombre, ahora: Veredicto::Falla, .. } if nombre == "suma")
    );
    assert_eq!(r.codigo_de_salida(), 1);
    let t = r.a_texto();
    assert!(
        t.contains(
            "SE ROMPE   suma: pasaba y ahora falla (registro A: esperado 0x01, obtenido 0x02)"
        ),
        "{t}"
    );
}

#[test]
fn una_prueba_que_ahora_da_error_tambien_se_rompe() {
    let antes = informe(vec![prueba("p", Veredicto::Pasa, 10)]);
    let mut e = prueba("p", Veredicto::Error, 0);
    e.motivo = Some("p.s:3:1: instruccion desconocida\nsegunda linea".into());
    let r = comparar_resultados(&antes, &informe(vec![e]), Umbral::default());
    assert!(r.rompe());
    assert!(r
        .a_texto()
        .contains("da error (p.s:3:1: instruccion desconocida)"));
}

#[test]
fn mas_ciclos_con_el_mismo_resultado_es_lentitud() {
    let antes = informe(vec![prueba("bucle", Veredicto::Pasa, 100)]);
    let ahora = informe(vec![prueba("bucle", Veredicto::Pasa, 130)]);
    let r = comparar_resultados(&antes, &ahora, Umbral::default());
    assert_eq!(
        r.cambios,
        vec![Cambio::MasLenta {
            nombre: "bucle".into(),
            antes: 100,
            ahora: 130
        }]
    );
    assert_eq!(r.codigo_de_salida(), 6);
    assert!(r
        .a_texto()
        .contains("MAS LENTA  bucle: 100 -> 130 ciclos (+30.0%)"));
}

#[test]
fn el_umbral_deja_pasar_aumentos_pequenos() {
    let antes = informe(vec![prueba("bucle", Veredicto::Pasa, 100)]);
    let ahora = informe(vec![prueba("bucle", Veredicto::Pasa, 105)]);
    assert!(
        comparar_resultados(&antes, &ahora, Umbral { porcentaje: 10.0 })
            .cambios
            .is_empty()
    );
    assert!(comparar_resultados(&antes, &ahora, Umbral { porcentaje: 4.0 }).mas_lentas());
}

#[test]
fn menos_ciclos_se_informa_pero_no_es_regresion() {
    let antes = informe(vec![prueba("a", Veredicto::Pasa, 100)]);
    let ahora = informe(vec![prueba("a", Veredicto::Pasa, 80)]);
    let r = comparar_resultados(&antes, &ahora, Umbral::default());
    assert!(matches!(r.cambios[0], Cambio::MasRapida { .. }));
    assert!(!r.hay_regresion());
}

#[test]
fn una_prueba_reparada_nueva_o_desaparecida_se_nota() {
    let antes = informe(vec![
        prueba("rota", Veredicto::Falla, 5),
        prueba("vieja", Veredicto::Pasa, 5),
    ]);
    let ahora = informe(vec![
        prueba("rota", Veredicto::Pasa, 5),
        prueba("nueva", Veredicto::Pasa, 5),
    ]);
    let r = comparar_resultados(&antes, &ahora, Umbral::default());
    let nombres: Vec<&str> = r.cambios.iter().map(Cambio::nombre).collect();
    assert_eq!(nombres, ["rota", "nueva", "vieja"]);
    assert!(matches!(r.cambios[0], Cambio::Repara { .. }));
    assert!(matches!(r.cambios[1], Cambio::Nueva { .. }));
    assert!(matches!(r.cambios[2], Cambio::Desaparecida { .. }));
    assert!(!r.hay_regresion());
}

#[test]
fn si_algo_se_rompe_y_algo_es_mas_lento_gana_la_ruptura() {
    let antes = informe(vec![
        prueba("a", Veredicto::Pasa, 10),
        prueba("b", Veredicto::Pasa, 10),
    ]);
    let ahora = informe(vec![
        prueba("a", Veredicto::Falla, 10),
        prueba("b", Veredicto::Pasa, 99),
    ]);
    let r = comparar_resultados(&antes, &ahora, Umbral::default());
    assert!(r.rompe() && r.mas_lentas());
    assert_eq!(r.codigo_de_salida(), 1);
}

#[test]
fn una_prueba_que_ya_fallaba_y_sigue_fallando_no_es_un_cambio() {
    let a = informe(vec![prueba("x", Veredicto::Falla, 10)]);
    let r = comparar_resultados(&a, &a.clone(), Umbral::default());
    assert!(r.cambios.is_empty());
    assert_eq!(r.iguales, 1);
}

#[test]
fn el_informe_de_regresion_viaja_como_json() {
    let antes = informe(vec![prueba("a", Veredicto::Pasa, 10)]);
    let ahora = informe(vec![prueba("a", Veredicto::Pasa, 20)]);
    let r = comparar_resultados(&antes, &ahora, Umbral::default());
    let t = serde_json::to_string(&r).unwrap();
    assert!(t.contains("\"tipo\":\"mas_lenta\""));
    let otro: lorenz_tst::regresion::InformeRegresion = serde_json::from_str(&t).unwrap();
    assert_eq!(otro, r);
}
