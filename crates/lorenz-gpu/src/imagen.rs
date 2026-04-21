//! Volcado del framebuffer a imagen (RF-GPU-004) e inspeccion de pixeles (RF-GPU-022).

use crate::superficie::{descomponer, Superficie};

/// Imagen PPM binaria (`P6`): la mas sencilla de escribir y la lee casi cualquier visor.
pub fn a_ppm(s: &Superficie, factor: u32) -> Vec<u8> {
    let f = factor.max(1);
    let mut salida = format!("P6\n{} {}\n255\n", s.ancho() * f, s.alto() * f).into_bytes();
    for y in 0..s.alto() * f {
        for x in 0..s.ancho() * f {
            let (r, g, b) = descomponer(s.pixel((x / f) as i64, (y / f) as i64).unwrap_or(0));
            salida.extend([r as u8, g as u8, b as u8]);
        }
    }
    salida
}

fn crc32(datos: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in datos {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(datos: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &d in datos {
        a = (a + d as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn trozo(salida: &mut Vec<u8>, tipo: &[u8; 4], datos: &[u8]) {
    salida.extend((datos.len() as u32).to_be_bytes());
    let mut cuerpo = tipo.to_vec();
    cuerpo.extend(datos);
    salida.extend(&cuerpo);
    salida.extend(crc32(&cuerpo).to_be_bytes());
}

/// Imagen PNG de 8 bits por canal RGB, ampliada `factor` veces. Los datos van sin comprimir
/// (bloques "stored" de deflate): pesa mas, pero no necesita dependencias.
pub fn a_png(s: &Superficie, factor: u32) -> Vec<u8> {
    let f = factor.max(1);
    let (ancho, alto) = (s.ancho() * f, s.alto() * f);
    let mut crudo = Vec::with_capacity(((ancho * 3 + 1) * alto) as usize);
    for y in 0..alto {
        crudo.push(0); // filtro: ninguno
        for x in 0..ancho {
            let (r, g, b) = descomponer(s.pixel((x / f) as i64, (y / f) as i64).unwrap_or(0));
            crudo.extend([r as u8, g as u8, b as u8]);
        }
    }
    let mut z = vec![0x78, 0x01];
    let mut bloques = crudo.chunks(65535).peekable();
    if bloques.peek().is_none() {
        z.extend([1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(bloque) = bloques.next() {
        z.push(u8::from(bloques.peek().is_none()));
        z.extend((bloque.len() as u16).to_le_bytes());
        z.extend((!(bloque.len() as u16)).to_le_bytes());
        z.extend(bloque);
    }
    z.extend(adler32(&crudo).to_be_bytes());
    let mut ihdr = Vec::new();
    ihdr.extend(ancho.to_be_bytes());
    ihdr.extend(alto.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    let mut salida = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    trozo(&mut salida, b"IHDR", &ihdr);
    trozo(&mut salida, b"IDAT", &z);
    trozo(&mut salida, b"IEND", &[]);
    salida
}

/// Describe un pixel: posicion, color y, si hay buffer de profundidad, su profundidad.
pub fn inspeccionar(s: &Superficie, x: i64, y: i64) -> String {
    match s.pixel(x, y) {
        None => format!(
            "({x}, {y}): fuera de la superficie de {}x{}",
            s.ancho(),
            s.alto()
        ),
        Some(c) => {
            let (r, g, b) = descomponer(c);
            let mut t = format!("({x}, {y}): #{c:06X} (r={r}, g={g}, b={b})");
            if let Some(z) = s.profundidad_en(x, y) {
                if z == crate::superficie::PROFUNDIDAD_LEJANA {
                    t.push_str(", z=sin dibujar");
                } else {
                    t.push_str(&format!(", z={z}"));
                }
            }
            t
        }
    }
}

/// Dibujo con caracteres de una region (`ancho`x`alto` desde `x`,`y`): un caracter por pixel
/// segun su brillo. Sirve para ver el resultado en una terminal.
pub fn a_texto(s: &Superficie) -> String {
    const ESCALA: &[u8] = b" .:-=+*#%@";
    let mut t = String::new();
    for y in 0..s.alto() as i64 {
        for x in 0..s.ancho() as i64 {
            let (r, g, b) = descomponer(s.pixel(x, y).unwrap_or(0));
            let brillo = (r * 299 + g * 587 + b * 114) / 1000;
            t.push(ESCALA[(brillo as usize * (ESCALA.len() - 1) + 127) / 255] as char);
        }
        t.push('\n');
    }
    t
}
