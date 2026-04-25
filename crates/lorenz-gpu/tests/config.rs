use lorenz_gpu::*;

#[test]
fn sin_seccion_no_hay_unidad() {
    assert_eq!(cargar_config("nombre = \"x\"\n").unwrap(), None);
}

#[test]
fn valores_por_omision() {
    let c = cargar_config("[gpu]\n").unwrap().unwrap();
    assert_eq!(c, ConfigGpu::default());
}

#[test]
fn lee_todos_los_campos() {
    let c = cargar_config(
        "[gpu]\ndispositivo = \"gpu0\"\nancho = 32\nalto = 16\ncolor = 16\nprofundidad = true\ncola = 4\ninterrupcion = 3\nciclos_comando = 2\nciclos_pixel = 3\nciclos_triangulo = 5\n",
    )
    .unwrap()
    .unwrap();
    assert_eq!(c.dispositivo.as_deref(), Some("gpu0"));
    assert_eq!((c.superficie.ancho, c.superficie.alto), (32, 16));
    assert_eq!(c.superficie.color, ProfundidadColor::Bits16);
    assert!(c.superficie.profundidad);
    assert_eq!((c.cola, c.interrupcion), (4, Some(3)));
    assert_eq!(
        c.costes,
        Costes {
            comando: 2,
            pixel: 3,
            triangulo: 5
        }
    );
}

#[test]
fn rechaza_valores_absurdos() {
    for (t, frag) in [
        ("[gpu]\nancho = 0\n", "resolucion"),
        ("[gpu]\nalto = 99999\n", "resolucion"),
        ("[gpu]\ncolor = 12\n", "8, 16 o 24"),
        ("[gpu]\ncola = 0\n", "cola"),
        ("[gpu]\nciclos_comando = 0\n", "al menos 1"),
        ("[gpu]\nanchura = 3\n", "anchura"),
    ] {
        let e = cargar_config(t).unwrap_err().to_string();
        assert!(e.contains(frag), "{t:?} -> {e}");
    }
}
