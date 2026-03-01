//! Horario del pipeline: riesgos de datos, de control y prediccion (RF-CIC-003 a RF-CIC-011).

use lorenz_cic::pipeline::{Causa, ConfigPipeline, Entrada, Etapa, Planificador, Prediccion};

fn etapas(nombres: &[&str]) -> Vec<Etapa> {
    nombres
        .iter()
        .map(|n| Etapa {
            nombre: n.to_string(),
            hace: vec![],
        })
        .collect()
}

fn cinco(adelantamiento: bool, prediccion: Prediccion) -> ConfigPipeline {
    ConfigPipeline {
        etapas: etapas(&["IF", "ID", "EX", "MEM", "WB"]),
        adelantamiento,
        prediccion,
        lectura: 1,
        ejecucion: 2,
        memoria: 3,
        escritura: 4,
        salto: 2,
        ciclos_declarados: false,
        ciclos_objetivo: 0,
    }
}

fn alu(dir: u64, escribe: &str, lee: &[&str]) -> Entrada {
    Entrada {
        direccion: dir,
        mnemonico: "OP".into(),
        longitud: 1,
        lee: lee.iter().map(|s| s.to_string()).collect(),
        escribe: vec![escribe.to_string()],
        coste_busqueda: 1,
        coste_memoria: 0,
        ciclos_declarados: 1,
        ..Default::default()
    }
}

fn salto(dir: u64, tomado: bool) -> Entrada {
    Entrada {
        direccion: dir,
        mnemonico: "JZ".into(),
        longitud: 1,
        es_salto: true,
        tomado,
        coste_busqueda: 1,
        ciclos_declarados: 1,
        ..Default::default()
    }
}

fn dato(r: &str) -> Causa {
    Causa::Datos { registro: r.into() }
}

#[test]
fn instrucciones_independientes_salen_una_por_ciclo() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::NoTomado));
    let h: Vec<_> = (0..4)
        .map(|i| p.programar(&alu(i, &format!("R{i}"), &[])))
        .collect();
    assert_eq!(h[0].entra, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(h[3].entra, vec![3, 4, 5, 6, 7, 8]);
    assert_eq!(h[3].sale(), 8);
}

#[test]
fn con_adelantamiento_una_dependencia_de_alu_no_espera() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::NoTomado));
    p.programar(&alu(0, "A", &[]));
    let b = p.programar(&alu(1, "B", &["A"]));
    assert_eq!(b.entra, vec![1, 2, 3, 4, 5, 6]);
    assert!(b.esperas().is_empty());
}

#[test]
fn sin_adelantamiento_hay_dos_detenciones_por_datos() {
    let mut p = Planificador::nuevo(cinco(false, Prediccion::NoTomado));
    p.programar(&alu(0, "A", &[]));
    let b = p.programar(&alu(1, "B", &["A"]));
    // Espera en la etapa de lectura hasta que A escribe en WB (ciclo 4).
    assert_eq!(b.entra[2], 5, "{:?}", b.entra);
    assert_eq!(b.esperas(), vec![(dato("A"), 2)]);
}

#[test]
fn una_carga_seguida_de_su_uso_cuesta_un_ciclo_aun_con_adelantamiento() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::NoTomado));
    let mut carga = alu(0, "A", &[]);
    carga.carga = true;
    carga.coste_memoria = 1;
    p.programar(&carga);
    let b = p.programar(&alu(1, "B", &["A"]));
    assert_eq!(b.esperas(), vec![(dato("A"), 1)]);
}

#[test]
fn un_salto_mal_predicho_descarta_dos_instrucciones_y_retrasa_la_siguiente() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::NoTomado));
    p.programar(&alu(0, "A", &[]));
    let s = p.programar(&salto(1, true));
    assert_eq!(s.prediccion, Some((false, false)));
    assert_eq!(s.descartadas, 2);
    let sig = p.programar(&alu(8, "B", &[]));
    // Sin el salto habria entrado en el ciclo 2; entra cuando el salto sale de EX.
    assert_eq!(sig.entra[0], s.entra[3]);
    assert_eq!(sig.entra[0], 4);
    assert_eq!(sig.esperas()[0].0, Causa::Control);
}

#[test]
fn un_salto_bien_predicho_no_cuesta_nada() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::NoTomado));
    let s = p.programar(&salto(0, false));
    assert_eq!(s.prediccion, Some((false, true)));
    let sig = p.programar(&alu(1, "B", &[]));
    assert_eq!(sig.entra[0], 1);
}

#[test]
fn siempre_tomado_acierta_los_saltos_que_se_toman() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::Tomado));
    let s = p.programar(&salto(0, true));
    assert_eq!(s.prediccion, Some((true, true)));
    assert_eq!(s.descartadas, 0);
}

#[test]
fn el_calculo_del_destino_puede_costar_ciclos() {
    let mut c = cinco(true, Prediccion::Tomado);
    c.ciclos_objetivo = 1;
    let mut p = Planificador::nuevo(c);
    let s = p.programar(&salto(0, true));
    let sig = p.programar(&alu(9, "B", &[]));
    assert_eq!(sig.entra[0], s.entra[1] + 1);
}

#[test]
fn un_bit_aprende_de_la_ultima_vez() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::UnBit));
    let a = p.programar(&salto(4, true));
    let b = p.programar(&salto(4, true));
    assert_eq!(a.prediccion, Some((false, false)));
    assert_eq!(b.prediccion, Some((true, true)));
}

#[test]
fn dos_bits_tarda_dos_veces_en_cambiar_de_opinion() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::DosBits));
    let r: Vec<_> = [true, true, true, false, true]
        .iter()
        .map(|t| p.programar(&salto(4, *t)).prediccion.unwrap().0)
        .collect();
    // Empieza en "debil no tomado": falla una vez, y un solo fallo no lo hace cambiar despues.
    assert_eq!(r, vec![false, true, true, true, true]);
}

#[test]
fn un_salto_indirecto_nunca_se_acierta() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::Tomado));
    let mut j = salto(0, true);
    j.incondicional = true;
    j.indirecto = true;
    assert_eq!(p.programar(&j).prediccion, Some((true, false)));
}

#[test]
fn la_etapa_lenta_detiene_a_las_anteriores() {
    let mut p = Planificador::nuevo(cinco(true, Prediccion::NoTomado));
    let mut lenta = alu(0, "A", &[]);
    lenta.carga = true;
    lenta.coste_memoria = 4;
    p.programar(&lenta);
    let b = p.programar(&alu(1, "B", &[]));
    let c = p.programar(&alu(2, "C", &[]));
    // B espera en EX a que la memoria quede libre; C espera detras de B.
    assert_eq!(b.esperas(), vec![(Causa::Estructural, 3)]);
    assert!(c.esperas().iter().all(|(k, _)| *k == Causa::Estructural));
}

#[test]
fn las_instrucciones_declaradas_de_varios_ciclos_alargan_la_ejecucion() {
    let mut c = cinco(true, Prediccion::NoTomado);
    c.ciclos_declarados = true;
    let mut p = Planificador::nuevo(c);
    let mut m = alu(0, "A", &[]);
    m.ciclos_declarados = 4;
    let h = p.programar(&m);
    assert_eq!(h.trabajo[2], 4);
    assert_eq!(h.sale(), 8);
}
