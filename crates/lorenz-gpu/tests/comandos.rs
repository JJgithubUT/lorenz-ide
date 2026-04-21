use lorenz_gpu::*;

fn todos() -> Vec<Comando> {
    let v = |x, y, color| Vertice { x, y, color };
    vec![
        Comando::Nada,
        Comando::Color(0x123456),
        Comando::Limpiar,
        Comando::Rellenar {
            x: -1,
            y: 2,
            ancho: 3,
            alto: 4,
        },
        Comando::Linea {
            x0: 0,
            y0: 1,
            x1: -2,
            y1: 3,
        },
        Comando::Triangulo {
            v: [(0, 0), (5, 0), (0, 5)],
        },
        Comando::TrianguloColor {
            v: [v(0, 0, 0xFF0000), v(5, 0, 0x00FF00), v(0, 5, 0x0000FF)],
        },
        Comando::Profundidad(9),
        Comando::PruebaZ(true),
        Comando::FinLote,
    ]
}

#[test]
fn cada_comando_vuelve_igual_tras_codificarlo() {
    for c in todos() {
        let p = c.a_palabras();
        assert_eq!(Comando::decodificar(&p).unwrap(), (c, p.len()), "{c:?}");
    }
}

#[test]
fn una_tira_se_decodifica_entera() {
    let p: Vec<i64> = todos().iter().flat_map(|c| c.a_palabras()).collect();
    assert_eq!(Comando::decodificar_todos(&p).unwrap(), todos());
}

#[test]
fn errores_claros() {
    assert_eq!(
        Comando::decodificar(&[99]).unwrap_err(),
        ErrorGpu::OperacionDesconocida { codigo: 99 }
    );
    assert_eq!(
        Comando::decodificar(&[3, 1, 2]).unwrap_err(),
        ErrorGpu::ComandoIncompleto {
            operacion: "rellenar",
            faltan: 2
        }
    );
    assert!(Comando::decodificar(&[]).is_err());
}

#[test]
fn el_guion_se_lee() {
    let g = leer_guion(
        "; fondo\ncolor #102030\nlimpiar\nrellenar 1 2 3 4 ; caja\nlinea 0 0 -3 5\nprueba_z 1\nfin_lote\n",
    )
    .unwrap();
    assert_eq!(g[0], Comando::Color(0x102030));
    assert_eq!(
        g[2],
        Comando::Rellenar {
            x: 1,
            y: 2,
            ancho: 3,
            alto: 4
        }
    );
    assert_eq!(
        g[3],
        Comando::Linea {
            x0: 0,
            y0: 0,
            x1: -3,
            y1: 5
        }
    );
    assert_eq!(g[4], Comando::PruebaZ(true));
    assert_eq!(g.len(), 6);
}

#[test]
fn el_guion_senala_la_linea_del_error() {
    let e = leer_guion("limpiar\nrellenar 1 2\n").unwrap_err();
    assert_eq!(
        e.to_string(),
        "linea 2: `rellenar` lleva 4 argumentos y se dieron 2"
    );
    let e = leer_guion("\n\ndibujar 3").unwrap_err();
    assert_eq!(e.to_string(), "linea 3: la orden `dibujar` no existe");
    let e = leer_guion("color rojo").unwrap_err();
    assert!(e.to_string().contains("no es un numero"));
}
