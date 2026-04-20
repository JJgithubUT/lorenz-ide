use lorenz_gpu::*;

fn sup(ancho: u32, alto: u32, z: bool) -> Superficie {
    Superficie::nueva(ConfigSuperficie {
        ancho,
        alto,
        color: ProfundidadColor::Bits24,
        profundidad: z,
    })
}

#[test]
fn rellenar_cubre_el_rectangulo_y_recorta() {
    let mut s = sup(8, 8, false);
    assert_eq!(rellenar(&mut s, 2, 2, 3, 2, 0xFF0000, None), 6);
    assert_eq!(s.pixel(2, 2), Some(0xFF0000));
    assert_eq!(s.pixel(4, 3), Some(0xFF0000));
    assert_eq!(s.pixel(5, 3), Some(0));
    assert_eq!(rellenar(&mut s, -2, -2, 4, 4, 0x00FF00, None), 4);
    assert_eq!(rellenar(&mut s, 6, 6, 10, 10, 1, None), 4);
}

#[test]
fn linea_incluye_los_extremos_en_cualquier_sentido() {
    let mut s = sup(8, 8, false);
    assert_eq!(linea(&mut s, 0, 0, 7, 0, 5, None), 8);
    assert_eq!(linea(&mut s, 7, 1, 0, 1, 5, None), 8);
    assert_eq!(linea(&mut s, 0, 2, 5, 7, 5, None), 6);
    assert_eq!(s.pixel(5, 7), Some(5));
    assert_eq!(linea(&mut s, 3, 3, 3, 3, 9, None), 1);
}

#[test]
fn triangulo_de_color_plano() {
    let mut s = sup(8, 8, false);
    let v = |x, y| Vertice {
        x,
        y,
        color: 0x112233,
    };
    let n = triangulo(&mut s, [v(0, 0), v(6, 0), v(0, 6)], false, None);
    assert_eq!(n, 28); // 7+6+5+4+3+2+1
    assert_eq!(s.pixel(1, 1), Some(0x112233));
    assert_eq!(s.pixel(6, 6), Some(0));
}

#[test]
fn triangulo_no_depende_del_orden_de_los_vertices() {
    let v = |x, y| Vertice { x, y, color: 7 };
    let mut a = sup(8, 8, false);
    let mut b = sup(8, 8, false);
    triangulo(&mut a, [v(0, 0), v(6, 0), v(0, 6)], false, None);
    triangulo(&mut b, [v(0, 6), v(0, 0), v(6, 0)], false, None);
    assert_eq!(a.pixeles(), b.pixeles());
}

#[test]
fn triangulo_degenerado_no_dibuja() {
    let mut s = sup(8, 8, false);
    let v = |x, y| Vertice { x, y, color: 1 };
    assert_eq!(
        triangulo(&mut s, [v(0, 0), v(3, 3), v(6, 6)], false, None),
        0
    );
}

#[test]
fn interpolacion_conserva_los_colores_de_los_vertices() {
    let mut s = sup(16, 16, false);
    let v = |x, y, color| Vertice { x, y, color };
    triangulo(
        &mut s,
        [v(0, 0, 0xFF0000), v(12, 0, 0x00FF00), v(0, 12, 0x0000FF)],
        true,
        None,
    );
    assert_eq!(s.pixel(0, 0), Some(0xFF0000));
    assert_eq!(s.pixel(12, 0), Some(0x00FF00));
    assert_eq!(s.pixel(0, 12), Some(0x0000FF));
    let (r, g, b) = descomponer(s.pixel(4, 4).unwrap());
    assert!(
        r > 0 && g > 0 && b > 0,
        "el interior mezcla los tres colores"
    );
}

#[test]
fn la_prueba_de_profundidad_deja_ver_lo_mas_cercano() {
    let mut s = sup(4, 4, true);
    rellenar(&mut s, 0, 0, 4, 4, 0xAA, Some(10));
    assert_eq!(
        rellenar(&mut s, 0, 0, 4, 4, 0xBB, Some(20)),
        0,
        "mas lejos: no se dibuja"
    );
    assert_eq!(s.pixel(1, 1), Some(0xAA));
    assert_eq!(
        rellenar(&mut s, 0, 0, 2, 2, 0xCC, Some(5)),
        4,
        "mas cerca: gana"
    );
    assert_eq!(s.pixel(1, 1), Some(0xCC));
    assert_eq!(s.pixel(3, 3), Some(0xAA));
    assert_eq!(s.profundidad_en(1, 1), Some(5));
}

#[test]
fn sin_buffer_de_profundidad_se_ignora_z() {
    let mut s = sup(2, 2, false);
    rellenar(&mut s, 0, 0, 2, 2, 1, Some(10));
    assert_eq!(rellenar(&mut s, 0, 0, 2, 2, 2, Some(99)), 4);
    assert_eq!(s.profundidad_en(0, 0), None);
}

#[test]
fn la_profundidad_de_color_redondea() {
    assert_eq!(ProfundidadColor::Bits24.cuantizar(0x123456), 0x123456);
    assert_eq!(ProfundidadColor::Bits16.cuantizar(0xFFFFFF), 0xFFFFFF);
    assert_eq!(ProfundidadColor::Bits8.cuantizar(0xFFFFFF), 0xFFFFFF);
    assert_eq!(ProfundidadColor::Bits8.cuantizar(0x000000), 0);
    let mut s = Superficie::nueva(ConfigSuperficie {
        ancho: 1,
        alto: 1,
        color: ProfundidadColor::Bits8,
        profundidad: false,
    });
    s.poner(0, 0, 0x80_80_80, None);
    assert_ne!(s.pixel(0, 0), Some(0x808080), "8 bits no guarda 24");
    assert_eq!(ProfundidadColor::desde_bits(12), None);
}
