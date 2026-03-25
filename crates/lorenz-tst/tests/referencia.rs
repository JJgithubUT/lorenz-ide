//! El evaluador de referencia de la semantica (RF-TST-017).

use std::path::PathBuf;

use lorenz_arch::modelo::Maquina;
use lorenz_tst::referencia::{
    banderas_asignadas, usa_puertos, ErrorRef, Referencia, ValorDeOperando,
};

fn maquina() -> Maquina {
    let ruta =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ejemplos/ejemplo-8/maquina.toml");
    lorenz_arch::carga::cargar_archivo(ruta).unwrap()
}

fn op(nombre: &str, valor: i128) -> ValorDeOperando {
    ValorDeOperando {
        nombre: nombre.into(),
        valor,
    }
}

fn sin_memoria(_: u64) -> u64 {
    0
}

#[test]
fn la_suma_con_acarreo_enciende_c_y_z() {
    let m = maquina();
    let r = Referencia::nueva(&m);
    let add = m.instruccion("ADD").unwrap();
    let mut e = r.estado_inicial();
    e.registros.insert("A".into(), 0xFF);
    e.registros.insert("B".into(), 1);
    // rd = A (indice 0), rs = B (indice 1) si los permitidos empiezan por A, B.
    let res = r
        .ejecutar(add, &[op("rd", 0), op("rs", 1)], e, 1, &sin_memoria)
        .unwrap();
    assert_eq!(res.estado.registros["A"], 0);
    assert!(res.estado.banderas["Z"]);
    assert!(res.estado.banderas["C"]);
    assert_eq!(res.ciclos, 2);
}

#[test]
fn una_instruccion_sin_semantica_solo_avanza_el_pc() {
    let m = maquina();
    let r = Referencia::nueva(&m);
    let nop = m.instruccion("NOP").unwrap();
    let res = r
        .ejecutar(nop, &[], r.estado_inicial(), 0x11, &sin_memoria)
        .unwrap();
    assert_eq!(res.estado.registros["PC"], 0x11);
    assert!(!res.estado.salto);
}

#[test]
fn un_salto_condicional_cambia_el_pc_y_los_ciclos_solo_si_se_toma() {
    let m = maquina();
    let r = Referencia::nueva(&m);
    let jz = m.instruccion("JZ").unwrap();
    let mut e = r.estado_inicial();
    e.banderas.insert("Z".into(), true);
    let tomado = r
        .ejecutar(jz, &[op("dir", 0x40)], e, 3, &sin_memoria)
        .unwrap();
    assert_eq!(tomado.estado.registros["PC"], 0x40);
    assert!(tomado.estado.salto);
    let no = r
        .ejecutar(jz, &[op("dir", 0x40)], r.estado_inicial(), 3, &sin_memoria)
        .unwrap();
    assert_eq!(no.estado.registros["PC"], 3);
    assert!(!no.estado.salto);
}

#[test]
fn una_escritura_en_memoria_se_anota_con_su_valor_truncado() {
    let m = maquina();
    let r = Referencia::nueva(&m);
    let st = m.instruccion("ST").unwrap();
    let mut e = r.estado_inicial();
    e.registros.insert("A".into(), 0x7E);
    let res = r
        .ejecutar(st, &[op("dir", 0x2000), op("rs", 0)], e, 3, &sin_memoria)
        .unwrap();
    assert_eq!(res.estado.escrituras[&0x2000], 0x7E);
    assert!(res.estado.accedidas.contains(&0x2000));
}

#[test]
fn una_lectura_pide_el_contenido_a_la_memoria_dada() {
    let m = maquina();
    let r = Referencia::nueva(&m);
    let ld = m.instruccion("LD").unwrap();
    let res = r
        .ejecutar(
            ld,
            &[op("rd", 2), op("dir", 0x2000)],
            r.estado_inicial(),
            3,
            &|d| if d == 0x2000 { 0x5A } else { 0 },
        )
        .unwrap();
    assert_eq!(res.estado.registros["C"], 0x5A);
    assert!(res.estado.lecturas_iniciales.contains(&0x2000));
}

#[test]
fn hlt_detiene() {
    let m = maquina();
    let r = Referencia::nueva(&m);
    let res = r
        .ejecutar(
            m.instruccion("HLT").unwrap(),
            &[],
            r.estado_inicial(),
            1,
            &sin_memoria,
        )
        .unwrap();
    assert!(res.estado.detenida);
}

#[test]
fn las_instrucciones_con_puertos_no_se_modelan() {
    let m = maquina();
    let r = Referencia::nueva(&m);
    let con_puertos: Vec<_> = m.instrucciones.iter().filter(|i| usa_puertos(i)).collect();
    for i in &con_puertos {
        let res = r.ejecutar(
            i,
            &[op("rs", 0), op("rd", 0), op("puerto", 0)],
            r.estado_inicial(),
            1,
            &sin_memoria,
        );
        assert_eq!(res.unwrap_err(), ErrorRef::UsaPuertos, "{}", i.mnemonico);
    }
}

#[test]
fn las_banderas_asignadas_a_mano_se_reconocen() {
    let m = maquina();
    for i in &m.instrucciones {
        let a = banderas_asignadas(i, &m);
        // En el ejemplo ninguna instruccion asigna una bandera por su nombre.
        assert!(a.is_empty(), "{}: {a:?}", i.mnemonico);
    }
}
