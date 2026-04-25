//! Las primitivas que sabe dibujar la unidad grafica (RF-GPU-005 a RF-GPU-009).
//!
//! Todas devuelven cuantos pixeles escribieron, que es lo que cuesta ciclos.

use crate::superficie::{componer, descomponer, Superficie};

/// Un vertice: posicion en pixeles y color `0xRRGGBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vertice {
    pub x: i64,
    pub y: i64,
    pub color: u32,
}

/// Rellena un rectangulo (RF-GPU-005). Lo que cae fuera de la superficie se recorta.
pub fn rellenar(
    s: &mut Superficie,
    x: i64,
    y: i64,
    ancho: i64,
    alto: i64,
    rgb: u32,
    z: Option<u32>,
) -> u64 {
    let mut escritos = 0;
    for yy in y.max(0)..(y + alto).min(s.alto() as i64) {
        for xx in x.max(0)..(x + ancho).min(s.ancho() as i64) {
            if s.poner(xx, yy, rgb, z) {
                escritos += 1;
            }
        }
    }
    escritos
}

/// Traza una linea de Bresenham, con ambos extremos incluidos (RF-GPU-006).
pub fn linea(
    s: &mut Superficie,
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    rgb: u32,
    z: Option<u32>,
) -> u64 {
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let (mut x, mut y, mut err) = (x0, y0, dx + dy);
    let mut escritos = 0;
    loop {
        if s.poner(x, y, rgb, z) {
            escritos += 1;
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    escritos
}

fn area2(a: Vertice, b: Vertice, p: (i64, i64)) -> i64 {
    (b.x - a.x) * (p.1 - a.y) - (b.y - a.y) * (p.0 - a.x)
}

/// Rasteriza un triangulo (RF-GPU-007, RF-GPU-008).
///
/// Con `interpolar` los tres colores se mezclan por coordenadas baricentricas; sin ella, todo el
/// triangulo lleva el color del primer vertice. Un pixel pertenece al triangulo si su centro esta
/// dentro o sobre una arista; un triangulo degenerado (area cero) no dibuja nada. El orden de los
/// vertices no importa.
pub fn triangulo(s: &mut Superficie, v: [Vertice; 3], interpolar: bool, z: Option<u32>) -> u64 {
    let [mut a, mut b, c] = v;
    let mut area = area2(a, b, (c.x, c.y));
    if area == 0 {
        return 0;
    }
    if area < 0 {
        std::mem::swap(&mut a, &mut b);
        area = -area;
    }
    let (min_x, max_x) = (
        a.x.min(b.x).min(c.x).max(0),
        a.x.max(b.x).max(c.x).min(s.ancho() as i64 - 1),
    );
    let (min_y, max_y) = (
        a.y.min(b.y).min(c.y).max(0),
        a.y.max(b.y).max(c.y).min(s.alto() as i64 - 1),
    );
    let mut escritos = 0;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (wa, wb, wc) = (
                area2(b, c, (x, y)),
                area2(c, a, (x, y)),
                area2(a, b, (x, y)),
            );
            if wa < 0 || wb < 0 || wc < 0 {
                continue;
            }
            let color = if interpolar {
                mezclar([a.color, b.color, c.color], [wa, wb, wc], area)
            } else {
                v[0].color
            };
            if s.poner(x, y, color, z) {
                escritos += 1;
            }
        }
    }
    escritos
}

fn mezclar(colores: [u32; 3], pesos: [i64; 3], total: i64) -> u32 {
    let mut canal = [0i64; 3];
    for (c, w) in colores.iter().zip(pesos) {
        let (r, g, b) = descomponer(*c);
        canal[0] += r as i64 * w;
        canal[1] += g as i64 * w;
        canal[2] += b as i64 * w;
    }
    let r = |v: i64| ((v + total / 2) / total) as u32;
    componer(r(canal[0]), r(canal[1]), r(canal[2]))
}
