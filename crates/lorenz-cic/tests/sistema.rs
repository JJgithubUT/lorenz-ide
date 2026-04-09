//! Varios nucleos sobre memoria compartida (RF-CIC-023).

mod comun;

use comun::*;
use lorenz_cic::{cargar_config, Sistema};
use lorenz_emu::Emulador;

/// LDI A, 42 ; ST A -> [0x2000] ; HLT
const ESCRIBE: [u8; 7] = [LDI_A, 42, 0x50, 0x00, 0x20, 0x00, HLT];

/// Lee [0x2000] en B tras `esperas` NOP, y termina.
fn lee_tras(esperas: usize) -> Vec<u8> {
    let mut p = vec![0x00; esperas];
    p.extend([0x44, 0x00, 0x20, HLT]);
    p
}

fn sistema(extra: &str, programas: &[&[u8]]) -> Sistema {
    let texto = format!("{}\n{extra}\n", texto_maquina());
    let cfg = cargar_config(&texto).expect("la temporizacion debe cargar");
    let emus = programas
        .iter()
        .map(|p| {
            let m = lorenz_arch::cargar(&texto).unwrap();
            let mut e = Emulador::nuevo(m).unwrap();
            let palabras: Vec<u64> = p.iter().map(|b| u64::from(*b)).collect();
            e.cargar_programa(0, &palabras).unwrap();
            e
        })
        .collect();
    Sistema::nuevo(emus, cfg).unwrap()
}

const DOS: &str = "[nucleos]\ncantidad = 2\ncompartida = [\"ram\"]\n";

fn registro_b(s: &Sistema, nucleo: usize) -> u64 {
    s.nucleo(nucleo).emulador().estado().registro("B").unwrap()
}

#[test]
fn lo_que_un_nucleo_escribe_lo_lee_otro_que_llega_despues() {
    let esperando = lee_tras(12);
    let mut s = sistema(DOS, &[&ESCRIBE, &esperando]);
    s.ejecutar(1000).unwrap();
    assert!(s.detenido());
    assert_eq!(registro_b(&s, 1), 42);
    assert_eq!(s.metricas().propagaciones, 1);
}

#[test]
fn un_nucleo_que_lee_antes_de_la_escritura_ve_el_valor_viejo() {
    let ya = lee_tras(0);
    let mut s = sistema(DOS, &[&ESCRIBE, &ya]);
    s.ejecutar(1000).unwrap();
    assert_eq!(registro_b(&s, 1), 0);
    // pero la memoria del segundo acaba con el valor escrito
    let m = s.nucleo(1).emulador().memoria();
    assert_eq!(m.leer_privilegiado(0x2000).unwrap(), 42);
}

#[test]
fn lo_que_no_esta_en_una_region_compartida_no_se_propaga() {
    // la ROM (0x0000) no se declara compartida; una escritura privada a la RAM de otro rango
    // tampoco: solo se comparte `ram` y aqui compartimos solo `video`
    let esperando = lee_tras(12);
    let mut s = sistema(
        "[nucleos]\ncantidad = 2\ncompartida = [\"video\"]\n",
        &[&ESCRIBE, &esperando],
    );
    s.ejecutar(1000).unwrap();
    assert_eq!(registro_b(&s, 1), 0, "la ram es privada de cada nucleo");
    assert_eq!(s.metricas().propagaciones, 0);
}

#[test]
fn el_contenido_inicial_de_lo_compartido_se_copia_del_primer_nucleo() {
    let texto = format!("{}\n{DOS}\n", texto_maquina());
    let cfg = cargar_config(&texto).unwrap();
    let mk = |dato: u64| {
        let m = lorenz_arch::cargar(&texto).unwrap();
        let mut e = Emulador::nuevo(m).unwrap();
        e.memoria_mut().escribir_privilegiado(0x2005, dato).unwrap();
        e
    };
    let s = Sistema::nuevo(vec![mk(7), mk(9)], cfg).unwrap();
    for i in 0..2 {
        assert_eq!(
            s.nucleo(i)
                .emulador()
                .memoria()
                .leer_privilegiado(0x2005)
                .unwrap(),
            7
        );
    }
}

#[test]
fn el_numero_de_emuladores_debe_ser_el_de_nucleos() {
    let texto = format!("{}\n{DOS}\n", texto_maquina());
    let cfg = cargar_config(&texto).unwrap();
    let m = lorenz_arch::cargar(&texto).unwrap();
    let e = Emulador::nuevo(m).unwrap();
    let r = Sistema::nuevo(vec![e], cfg);
    assert!(r.err().unwrap().contains("2 nucleos"));
}

#[test]
fn sin_seccion_de_nucleos_es_un_sistema_de_uno_y_da_lo_mismo_que_el_simulador() {
    let mut s = sistema(CINCO, &[&[ADD_AB, ADD_AB, HLT]]);
    s.ejecutar(100).unwrap();
    let mut solo = simulador(CINCO, &[ADD_AB, ADD_AB, HLT]);
    solo.ejecutar(100).unwrap();
    assert_eq!(s.metricas().nucleos[0], solo.metricas());
    assert_eq!(s.metricas().ciclos, solo.metricas().ciclos);
}

#[test]
fn el_resultado_se_repite() {
    let esperando = lee_tras(5);
    let a = {
        let mut s = sistema(DOS, &[&ESCRIBE, &esperando]);
        s.ejecutar(1000).unwrap();
        s.metricas()
    };
    let b = {
        let mut s = sistema(DOS, &[&ESCRIBE, &esperando]);
        s.ejecutar(1000).unwrap();
        s.metricas()
    };
    assert_eq!(a, b);
}

#[test]
fn los_ciclos_del_sistema_son_los_del_nucleo_que_termina_el_ultimo() {
    let largo = lee_tras(30);
    let mut s = sistema(DOS, &[&ESCRIBE, &largo]);
    s.ejecutar(1000).unwrap();
    let m = s.metricas();
    assert_eq!(m.ciclos, m.nucleos[0].ciclos.max(m.nucleos[1].ciclos));
    assert!(m.nucleos[1].ciclos > m.nucleos[0].ciclos);
    assert_eq!(
        m.instrucciones,
        m.nucleos[0].instrucciones + m.nucleos[1].instrucciones
    );
}

#[test]
fn el_limite_cuenta_instrucciones_de_todos_los_nucleos() {
    let largo = lee_tras(30);
    let mut s = sistema(DOS, &[&ESCRIBE, &largo]);
    assert_eq!(s.ejecutar(10).unwrap(), 10);
    assert!(!s.detenido());
}

#[test]
fn un_bus_compartido_entre_nucleos_los_hace_esperar() {
    let p = lee_tras(0);
    let con = format!("{CINCO}{DOS}[bus]\ncompartido = true\n");
    let sin = format!("{CINCO}{DOS}");
    let mut a = sistema(&con, &[&p, &p]);
    a.ejecutar(1000).unwrap();
    let mut b = sistema(&sin, &[&p, &p]);
    b.ejecutar(1000).unwrap();
    let (ma, mb) = (a.metricas(), b.metricas());
    assert!(ma.ciclos > mb.ciclos, "{} vs {}", ma.ciclos, mb.ciclos);
    assert!(ma.ciclos_de_bus_comun > 0);
    assert_eq!(mb.ciclos_de_bus_comun, 0);
    let esperas: u64 = ma.nucleos.iter().map(|m| m.esperas_bus).sum();
    assert!(esperas > 0);
}
