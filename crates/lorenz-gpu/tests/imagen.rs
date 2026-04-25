use lorenz_gpu::imagen::*;
use lorenz_gpu::*;

fn s() -> Superficie {
    let mut s = Superficie::nueva(ConfigSuperficie {
        ancho: 2,
        alto: 2,
        color: ProfundidadColor::Bits24,
        profundidad: true,
    });
    s.poner(1, 0, 0xFF8000, Some(7));
    s
}

#[test]
fn ppm_lleva_cabecera_y_pixeles() {
    let b = a_ppm(&s(), 1);
    assert!(b.starts_with(b"P6\n2 2\n255\n"));
    let datos = &b[b"P6\n2 2\n255\n".len()..];
    assert_eq!(datos.len(), 12);
    assert_eq!(&datos[3..6], &[0xFF, 0x80, 0x00]);
}

#[test]
fn ppm_ampliado_repite_los_pixeles() {
    let b = a_ppm(&s(), 3);
    assert!(b.starts_with(b"P6\n6 6\n255\n"));
    let datos = &b[b"P6\n6 6\n255\n".len()..];
    assert_eq!(datos.len(), 6 * 6 * 3);
    assert_eq!(&datos[3 * 3..3 * 3 + 3], &[0xFF, 0x80, 0x00]);
    assert_eq!(&datos[5 * 3..5 * 3 + 3], &[0xFF, 0x80, 0x00]);
}

#[test]
fn png_tiene_firma_y_trozos_validos() {
    let b = a_png(&s(), 1);
    assert_eq!(&b[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    assert_eq!(&b[12..16], b"IHDR");
    assert_eq!(&b[b.len() - 8..b.len() - 4], b"IEND");
    // crc de IEND es una constante conocida
    assert_eq!(&b[b.len() - 4..], &[0xAE, 0x42, 0x60, 0x82]);
}

#[test]
fn png_grande_parte_los_datos_en_bloques() {
    let g = Superficie::nueva(ConfigSuperficie {
        ancho: 200,
        alto: 200,
        color: ProfundidadColor::Bits24,
        profundidad: false,
    });
    let b = a_png(&g, 1);
    assert!(b.len() > 120_000);
}

#[test]
fn inspeccionar_describe_el_pixel() {
    let s = s();
    assert_eq!(
        inspeccionar(&s, 1, 0),
        "(1, 0): #FF8000 (r=255, g=128, b=0), z=7"
    );
    assert!(inspeccionar(&s, 0, 0).ends_with("z=sin dibujar"));
    assert!(inspeccionar(&s, 5, 5).contains("fuera"));
}

#[test]
fn texto_marca_el_brillo() {
    let t = a_texto(&s());
    assert_eq!(t.lines().count(), 2);
    assert_ne!(t.lines().next().unwrap().chars().nth(1), Some(' '));
}
