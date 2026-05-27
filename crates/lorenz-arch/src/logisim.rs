//! Exportacion de una maquina a un circuito de Logisim (RF-ECO-036).
//!
//! Genera un archivo `.circ` con los bloques de la maquina: un registro por cada registro, una ROM
//! con el programa, una RAM y un reloj, mas notas con la tabla de instrucciones. Los bloques **no
//! vienen cableados**: es el punto de partida para construir la ruta de datos a mano, con las
//! anchuras y el contenido de memoria ya puestos.

use std::fmt::Write;

use crate::modelo::Maquina;

fn xml(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Contenido de una memoria en el formato de Logisim: cabecera y palabras en hexadecimal.
fn contenido(tramos: &[(u64, Vec<u64>)], direccion: u32, palabra: u32) -> String {
    let limite = 1u64 << direccion;
    let mut ultimo = 0u64;
    for (d, p) in tramos {
        if !p.is_empty() {
            ultimo = ultimo.max((d + p.len() as u64 - 1).min(limite - 1));
        }
    }
    let mut imagen = vec![
        0u64;
        if tramos.iter().all(|t| t.1.is_empty()) {
            0
        } else {
            ultimo as usize + 1
        }
    ];
    for (d, p) in tramos {
        for (k, w) in p.iter().enumerate() {
            if let Some(c) = imagen.get_mut((d + k as u64) as usize) {
                *c = *w;
            }
        }
    }
    let mut s = format!("addr/data: {direccion} {palabra}\n");
    for fila in imagen.chunks(8) {
        let l: Vec<String> = fila.iter().map(|w| format!("{w:x}")).collect();
        writeln!(s, "{}", l.join(" ")).unwrap();
    }
    s
}

pub fn exportar(m: &Maquina, programa: &[(u64, Vec<u64>)]) -> String {
    let anchura_memoria = m.bits_direccion.min(16);
    let mut o = String::new();
    o.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n<project source=\"3.8.0\" version=\"1.0\">\n");
    writeln!(
        o,
        "  <!-- {} -->",
        xml(&crate::marca_generado("lorenz exportar logisim"))
    )
    .unwrap();
    for (i, l) in ["#Wiring", "#Gates", "#Plexers", "#Arithmetic", "#Memory"]
        .iter()
        .enumerate()
    {
        writeln!(o, "  <lib desc=\"{l}\" name=\"{i}\"/>").unwrap();
    }
    o.push_str("  <main name=\"main\"/>\n  <circuit name=\"main\">\n");
    let nota = |o: &mut String, x: i32, y: i32, texto: &str| {
        writeln!(o, "    <comp lib=\"0\" loc=\"({x},{y})\" name=\"Text\">\n      <a name=\"text\" val=\"{}\"/>\n    </comp>", xml(texto)).unwrap();
    };
    nota(
        &mut o,
        200,
        40,
        &format!("{}: bloques generados por Lorenz, sin cablear", m.nombre),
    );
    writeln!(o, "    <comp lib=\"0\" loc=\"(100,120)\" name=\"Clock\"/>").unwrap();
    writeln!(
        o,
        "    <comp lib=\"4\" loc=\"(400,120)\" name=\"ROM\">\n      <a name=\"addrWidth\" val=\"{anchura_memoria}\"/>\n      <a name=\"dataWidth\" val=\"{}\"/>\n      <a name=\"label\" val=\"programa\"/>\n      <a name=\"contents\">{}</a>\n    </comp>",
        m.bits_palabra,
        xml(&contenido(programa, anchura_memoria, m.bits_palabra))
    )
    .unwrap();
    writeln!(
        o,
        "    <comp lib=\"4\" loc=\"(700,120)\" name=\"RAM\">\n      <a name=\"addrWidth\" val=\"{anchura_memoria}\"/>\n      <a name=\"dataWidth\" val=\"{}\"/>\n      <a name=\"label\" val=\"ram\"/>\n    </comp>",
        m.bits_palabra
    )
    .unwrap();
    for (i, r) in m.registros.iter().enumerate() {
        let (x, y) = (200 + 150 * (i as i32 % 4), 300 + 120 * (i as i32 / 4));
        writeln!(
            o,
            "    <comp lib=\"4\" loc=\"({x},{y})\" name=\"Register\">\n      <a name=\"width\" val=\"{}\"/>\n      <a name=\"label\" val=\"{}\"/>\n    </comp>",
            r.bits,
            xml(&r.nombre)
        )
        .unwrap();
    }
    let base = 300 + 120 * (m.registros.len() as i32 / 4 + 1);
    for (i, ins) in m.instrucciones.iter().enumerate() {
        nota(
            &mut o,
            300,
            base + 20 * i as i32,
            &format!("{}  [{}]", ins.sintaxis, ins.codificacion.texto()),
        );
    }
    o.push_str("  </circuit>\n</project>\n");
    o
}
