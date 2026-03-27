//! Cache asociativa: aciertos, reemplazo y escritura (RF-CIC-015 a RF-CIC-019).

use lorenz_cic::{Cache, ConfigCache, Escritura, Reemplazo};

fn cfg(tamano: u64, linea: u64, vias: u64, r: Reemplazo, e: Escritura) -> ConfigCache {
    ConfigCache {
        tamano,
        linea,
        vias,
        reemplazo: r,
        escritura: e,
        latencia_acierto: 1,
        instrucciones: false,
    }
}

fn directa() -> Cache {
    Cache::nueva(cfg(8, 2, 1, Reemplazo::Lru, Escritura::PostEscritura)).unwrap()
}

#[test]
fn la_forma_se_valida() {
    assert!(Cache::nueva(cfg(0, 2, 1, Reemplazo::Lru, Escritura::PostEscritura)).is_err());
    assert!(Cache::nueva(cfg(10, 4, 1, Reemplazo::Lru, Escritura::PostEscritura)).is_err());
    assert!(Cache::nueva(cfg(8, 2, 3, Reemplazo::Lru, Escritura::PostEscritura)).is_err());
}

#[test]
fn el_primer_acceso_falla_y_los_vecinos_de_la_linea_aciertan() {
    let mut c = directa();
    assert!(!c.acceder(4, false).acierto);
    assert!(c.acceder(5, false).acierto, "misma linea");
    assert!(!c.acceder(6, false).acierto, "linea siguiente");
    let e = c.estadisticas();
    assert_eq!((e.fallos_lectura, e.aciertos_lectura), (2, 1));
}

#[test]
fn lectura_y_escritura_se_cuentan_por_separado() {
    let mut c = directa();
    c.acceder(0, true);
    c.acceder(1, true);
    c.acceder(0, false);
    let e = c.estadisticas();
    assert_eq!((e.fallos_escritura, e.aciertos_escritura), (1, 1));
    assert_eq!((e.fallos_lectura, e.aciertos_lectura), (0, 1));
    assert_eq!(e.tasa_de_aciertos(), Some(2.0 / 3.0));
}

#[test]
fn dos_lineas_del_mismo_conjunto_se_expulsan_en_una_cache_directa() {
    let mut c = directa();
    // 4 conjuntos de 2 palabras: las direcciones 0 y 8 caen en el conjunto 0.
    c.acceder(0, false);
    c.acceder(8, false);
    assert!(!c.acceder(0, false).acierto);
}

#[test]
fn con_dos_vias_conviven_y_lru_expulsa_la_menos_usada() {
    let mut c = Cache::nueva(cfg(8, 2, 2, Reemplazo::Lru, Escritura::PostEscritura)).unwrap();
    // 2 conjuntos: 0, 4 y 8 comparten el conjunto 0.
    c.acceder(0, false);
    c.acceder(4, false);
    assert!(c.acceder(0, false).acierto);
    c.acceder(8, false); // expulsa 4, que lleva mas tiempo sin usarse
    assert!(c.acceder(0, false).acierto);
    assert!(!c.acceder(4, false).acierto);
}

#[test]
fn fifo_expulsa_la_mas_antigua_aunque_se_haya_usado() {
    let mut c = Cache::nueva(cfg(8, 2, 2, Reemplazo::Fifo, Escritura::PostEscritura)).unwrap();
    c.acceder(0, false);
    c.acceder(4, false);
    c.acceder(0, false);
    c.acceder(8, false); // expulsa 0, que entro primero
    assert!(!c.acceder(0, false).acierto);
}

#[test]
fn el_reemplazo_aleatorio_es_determinista() {
    let correr = || {
        let mut c =
            Cache::nueva(cfg(8, 2, 2, Reemplazo::Aleatorio, Escritura::PostEscritura)).unwrap();
        (0..40u64)
            .map(|i| c.acceder((i * 4) % 24, false).acierto)
            .collect::<Vec<_>>()
    };
    assert_eq!(correr(), correr());
}

#[test]
fn post_escritura_vuelca_la_linea_sucia_al_expulsarla() {
    let mut c = directa();
    assert!(!c.acceder(0, true).volcado);
    let r = c.acceder(8, false);
    assert!(r.relleno && r.volcado);
    assert_eq!(c.estadisticas().volcados, 1);
}

#[test]
fn una_linea_limpia_no_se_vuelca() {
    let mut c = directa();
    c.acceder(0, false);
    assert!(!c.acceder(8, false).volcado);
}

#[test]
fn escritura_directa_escribe_siempre_en_memoria_y_no_asigna_en_el_fallo() {
    let mut c = Cache::nueva(cfg(8, 2, 1, Reemplazo::Lru, Escritura::EscrituraDirecta)).unwrap();
    let r = c.acceder(0, true);
    assert!(!r.acierto && !r.relleno && r.escritura_directa);
    assert!(!c.acceder(0, false).acierto, "no se trajo la linea");
    let r = c.acceder(0, true);
    assert!(r.acierto && r.escritura_directa);
    // Nunca queda sucia: expulsarla no vuelca nada.
    assert!(!c.acceder(8, false).volcado);
}

#[test]
fn el_contenido_muestra_etiquetas_y_bits() {
    let mut c = directa();
    c.acceder(9, true);
    let filas = c.contenido();
    assert_eq!(filas.len(), 4);
    let f = filas.iter().find(|f| f.valida).unwrap();
    assert_eq!((f.conjunto, f.via, f.sucia), (0, 0, true));
    assert_eq!(f.direccion, Some(8));
    assert_eq!(f.etiqueta, 1);
    assert!(c.volcar().contains("0 0 1 1 0x1 0x8"), "{}", c.volcar());
}
