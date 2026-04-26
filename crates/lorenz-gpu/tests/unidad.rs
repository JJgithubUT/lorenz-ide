use lorenz_gpu::*;

fn gpu(extra: &str) -> Gpu {
    let t = format!("[gpu]\nancho = 8\nalto = 8\n{extra}");
    Gpu::nueva(cargar_config(&t).unwrap().unwrap())
}

#[test]
fn un_comando_cuesta_su_base_mas_los_pixeles() {
    let mut g = gpu("");
    g.enviar(Comando::Rellenar {
        x: 0,
        y: 0,
        ancho: 2,
        alto: 2,
    })
    .unwrap();
    assert!(g.ocupada());
    g.avanzar(10_000);
    let m = g.metricas();
    assert_eq!(m.pixeles_escritos, 4);
    assert_eq!(m.ciclos_ocupada, 4 + 4); // base 4 + 4 pixeles a 1
    assert_eq!(m.ciclos_totales, 10_000);
    assert!(!g.ocupada());
    assert!(m.utilizacion() < 0.01);
}

#[test]
fn el_triangulo_paga_su_preparacion() {
    let mut g = gpu("");
    g.enviar(Comando::Triangulo {
        v: [(0, 0), (4, 0), (0, 4)],
    })
    .unwrap();
    g.avanzar(1000);
    let m = g.metricas();
    assert_eq!(m.triangulos, 1);
    assert_eq!(m.ciclos_ocupada, 4 + 8 + m.pixeles_escritos);
}

#[test]
fn la_cola_tiene_profundidad_y_rechaza() {
    let mut g = gpu("cola = 2\n");
    assert!(g.enviar(Comando::Nada).is_ok());
    assert!(g.enviar(Comando::Nada).is_ok());
    assert_eq!(
        g.enviar(Comando::Nada),
        Err(ErrorGpu::ColaLlena { profundidad: 2 })
    );
    assert_eq!(g.metricas().rechazados, 1);
    assert_eq!(g.metricas().cola_maxima, 2);
    g.avanzar(5); // termina el primero y toma el segundo
    assert_eq!(g.en_cola(), 0);
    assert!(g.enviar(Comando::Nada).is_ok());
}

#[test]
fn el_tiempo_se_reparte_entre_llamadas() {
    let mut a = gpu("");
    let mut b = gpu("");
    for g in [&mut a, &mut b] {
        g.enviar(Comando::Rellenar {
            x: 0,
            y: 0,
            ancho: 8,
            alto: 8,
        })
        .unwrap();
    }
    a.avanzar(68);
    for _ in 0..68 {
        b.avanzar(1);
    }
    assert_eq!(a.metricas(), b.metricas());
    assert_eq!(a.superficie(), b.superficie());
}

#[test]
fn fin_de_lote_avisa_una_vez() {
    let mut g = gpu("");
    g.enviar(Comando::Color(5)).unwrap();
    g.enviar(Comando::FinLote).unwrap();
    g.enviar(Comando::FinLote).unwrap();
    assert_eq!(g.tomar_lotes_terminados(), 0);
    g.avanzar(100);
    assert_eq!(g.tomar_lotes_terminados(), 2);
    assert_eq!(g.tomar_lotes_terminados(), 0);
    assert_eq!(g.metricas().lotes, 2);
}

#[test]
fn el_color_y_la_profundidad_se_mantienen_entre_comandos() {
    let mut g = gpu("profundidad = true\n");
    for c in [
        Comando::PruebaZ(true),
        Comando::Color(0xAA),
        Comando::Profundidad(10),
        Comando::Rellenar {
            x: 0,
            y: 0,
            ancho: 8,
            alto: 8,
        },
        Comando::Color(0xBB),
        Comando::Profundidad(20),
        Comando::Rellenar {
            x: 0,
            y: 0,
            ancho: 8,
            alto: 8,
        },
        Comando::Profundidad(1),
        Comando::Rellenar {
            x: 0,
            y: 0,
            ancho: 2,
            alto: 1,
        },
    ] {
        g.enviar(c).unwrap();
    }
    g.vaciar(100_000);
    assert_eq!(g.superficie().pixel(5, 5), Some(0xAA), "lo lejano no tapa");
    assert_eq!(g.superficie().pixel(0, 0), Some(0xBB), "lo cercano tapa");
}

#[test]
fn prueba_z_sin_buffer_se_ignora() {
    let mut g = gpu("");
    for c in [
        Comando::PruebaZ(true),
        Comando::Color(1),
        Comando::Profundidad(5),
        Comando::Rellenar {
            x: 0,
            y: 0,
            ancho: 1,
            alto: 1,
        },
        Comando::Color(2),
        Comando::Profundidad(9),
        Comando::Rellenar {
            x: 0,
            y: 0,
            ancho: 1,
            alto: 1,
        },
    ] {
        g.enviar(c).unwrap();
    }
    g.vaciar(1000);
    assert_eq!(g.superficie().pixel(0, 0), Some(2));
}

#[test]
fn limpiar_usa_el_color_actual() {
    let mut g = gpu("");
    g.enviar(Comando::Color(0x30)).unwrap();
    g.enviar(Comando::Limpiar).unwrap();
    g.vaciar(1000);
    assert!(g.superficie().pixeles().iter().all(|p| *p == 0x30));
    assert_eq!(g.metricas().pixeles_escritos, 64);
}

#[test]
fn vaciar_respeta_el_limite() {
    let mut g = gpu("");
    g.enviar(Comando::Limpiar).unwrap();
    assert_eq!(g.vaciar(10), 10);
    assert!(g.ocupada());
}
