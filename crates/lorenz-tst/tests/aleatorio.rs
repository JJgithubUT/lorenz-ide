//! Programas aleatorios, comparacion y reduccion (RF-TST-024 a RF-TST-026, RF-TST-035).

use std::path::PathBuf;

use lorenz_arch::modelo::Maquina;
use lorenz_emu::{Ejecutor, ErrorEjecucion, Paso, Vista};
use lorenz_tst::aleatorio::{
    ejecutar_aleatorio, ejecutar_aleatorio_con, emulador_para, generar_programa, reducir,
    referencia_para, Aleatorio, InformeAleatorio, Opciones,
};

fn maquina(ruta: &str) -> Maquina {
    lorenz_arch::carga::cargar_archivo(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ruta))
        .unwrap()
}

fn ejemplo() -> Maquina {
    maquina("../../ejemplos/ejemplo-8/maquina.toml")
}

fn rica() -> Maquina {
    maquina("tests/maquinas/rica-8.toml")
}

#[test]
fn el_generador_de_numeros_es_reproducible() {
    let mut a = Aleatorio::nuevo(42);
    let mut b = Aleatorio::nuevo(42);
    let va: Vec<u64> = (0..10).map(|_| a.siguiente()).collect();
    let vb: Vec<u64> = (0..10).map(|_| b.siguiente()).collect();
    assert_eq!(va, vb);
    let mut c = Aleatorio::nuevo(43);
    assert_ne!(va[0], c.siguiente());
    let mut r = Aleatorio::nuevo(1);
    for _ in 0..1000 {
        let x = r.entre(-3, 3);
        assert!((-3..=3).contains(&x));
        assert!(r.por_debajo_de(5) < 5);
    }
}

#[test]
fn la_misma_semilla_da_el_mismo_programa_y_otra_da_otro() {
    let m = rica();
    let g = |s| generar_programa(&m, &mut Aleatorio::nuevo(s), 15).unwrap();
    assert_eq!(g(7), g(7));
    assert_ne!(g(7), g(8));
    assert_eq!(g(7).instrucciones.len(), 15);
}

#[test]
fn un_programa_aleatorio_termina_con_la_instruccion_que_detiene() {
    let m = rica();
    let p = generar_programa(&m, &mut Aleatorio::nuevo(3), 10).unwrap();
    let texto = p.a_texto(&m);
    assert_eq!(texto.len(), 11);
    assert_eq!(texto.last().unwrap(), "HLT");
    // Sin saltos ni puertos.
    for t in &texto {
        assert!(!t.starts_with("BRA") && !t.starts_with("BNE") && !t.starts_with("CALL"));
        assert!(!t.starts_with("IN ") && !t.starts_with("OUT"));
    }
}

#[test]
fn el_emulador_y_la_referencia_coinciden_en_cientos_de_programas() {
    for m in [ejemplo(), rica()] {
        let i = ejecutar_aleatorio(
            &m,
            &Opciones {
                semilla: 2026,
                programas: 300,
                longitud: 25,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(i.divergencias.is_empty(), "{}", i.a_texto());
        assert!(i.instrucciones_comparadas > 300, "{}", i.a_texto());
    }
}

#[test]
fn el_informe_es_identico_con_la_misma_semilla() {
    let m = rica();
    let o = Opciones {
        semilla: 11,
        programas: 20,
        ..Default::default()
    };
    let a = ejecutar_aleatorio(&m, &o).unwrap();
    let b = ejecutar_aleatorio(&m, &o).unwrap();
    assert_eq!(a, b);
    // Los programas de una semilla se numeran desde ella: con `programas = 1` se repite el k-esimo.
    let sola = ejecutar_aleatorio(
        &m,
        &Opciones {
            semilla: 15,
            programas: 1,
            ..o
        },
    )
    .unwrap();
    assert_eq!(sola.programas, 1);
    assert!(sola.divergencias.is_empty());
}

#[test]
fn el_informe_viaja_como_json() {
    let m = ejemplo();
    let i = ejecutar_aleatorio(
        &m,
        &Opciones {
            programas: 5,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(InformeAleatorio::desde_json(&i.a_json()).unwrap(), i);
    assert!(InformeAleatorio::desde_json("{}").is_err());
}

/// Un emulador con un defecto: tras ejecutar un ADD informa mal el primer registro.
struct ConDefecto {
    dentro: lorenz_emu::Emulador,
    ultimo: String,
}

impl Ejecutor for ConDefecto {
    fn paso(&mut self) -> Result<Option<Paso>, ErrorEjecucion> {
        let r = self.dentro.paso()?;
        if let Some(p) = &r {
            self.ultimo = p.mnemonico.clone();
        }
        Ok(r)
    }

    fn vista(&self) -> Vista {
        let mut v = self.dentro.vista();
        if self.ultimo == "ADD" {
            v.registros[0].1 ^= 1;
        }
        v
    }
}

#[test]
fn una_implementacion_defectuosa_se_detecta_y_el_programa_se_reduce() {
    let m = ejemplo();
    let primera = |p: &_| -> Result<Box<dyn Ejecutor + '_>, String> {
        Ok(Box::new(ConDefecto {
            dentro: emulador_para(&m, p)?,
            ultimo: String::new(),
        }))
    };
    let segunda =
        |p: &_| -> Result<Box<dyn Ejecutor + '_>, String> { Ok(Box::new(referencia_para(&m, p)?)) };
    let i = ejecutar_aleatorio_con(
        &m,
        &Opciones {
            semilla: 5,
            programas: 30,
            longitud: 20,
            ..Default::default()
        },
        &primera,
        &segunda,
    )
    .unwrap();
    assert!(!i.divergencias.is_empty());
    assert_eq!(i.codigo_de_salida(), 1);
    for d in &i.divergencias {
        assert!(d.programa_reducido.len() < d.programa.len(), "{d:?}");
        // El minimo para ver el defecto: un ADD (y el HLT final).
        assert!(d.programa_reducido.len() <= 2, "{:?}", d.programa_reducido);
        assert!(d.programa_reducido.iter().any(|l| l.starts_with("ADD")));
    }
    let t = i.a_texto();
    assert!(t.contains("programa reducido"));
    assert!(t.contains("DIVERGE (semilla 5)") || t.contains("DIVERGE (semilla"));
}

#[test]
fn reducir_deja_lo_minimo_que_sigue_fallando() {
    // El fallo aparece cuando estan el 3 y el 7.
    let mut llamadas = 0;
    let r = reducir((0..20).collect::<Vec<i32>>(), &mut |s: &[i32]| {
        llamadas += 1;
        s.contains(&3) && s.contains(&7)
    });
    assert_eq!(r, vec![3, 7]);
    assert!(llamadas < 200);
}

#[test]
fn reducir_puede_dejar_la_lista_vacia() {
    let r = reducir(vec![1, 2, 3], &mut |_: &[i32]| true);
    assert!(r.is_empty());
}

#[test]
fn reducir_una_lista_que_no_falla_sin_ella_la_conserva() {
    let r = reducir(vec![1, 2, 3], &mut |s: &[i32]| s.contains(&2));
    assert_eq!(r, vec![2]);
}
