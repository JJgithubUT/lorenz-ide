//! La documentacion generada a partir de la descripcion (RF-ECO-017 a RF-ECO-023, RF-ARQ-036).

use lorenz_arch::documentar::{documentar, Comentarios};

const TEXTO: &str = r#"# Maquina de prueba
#
# Sirve para comprobar la documentacion.

[maquina]
nombre = "demo"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[flags]
# Se enciende si el resultado es cero.
Z = { bit = 0, descripcion = "cero" }
C = { bit = 1, descripcion = "acarreo" }

# Acumulador principal.
[[registro]]
nombre = "A"
bits = 8
rol = "general"

[[registro]]
nombre = "B"
bits = 8

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[registro]]
nombre = "F"
bits = 8
rol = "flags"

[[par]]
nombre = "AB"
alto = "A"
bajo = "B"

# Suma dos registros.
# Deja el resultado en el primero.
[[instruccion]]
mnemonico = "ADD"
sintaxis = "ADD {rd}, {rs}"
codigo = "0011 rd:2 rs:2"
semantica = "rd = rd + rs"
afecta = ["Z", "C"]
ciclos = 2
descripcion = "Suma."
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "B"]
  [[instruccion.operando]]
  nombre = "rs"
  tipo = "registro"
  de = ["A", "B"]

[[instruccion]]
mnemonico = "HLT"
sintaxis = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[instruccion]]
mnemonico = "JZ"
sintaxis = "JZ {dir}"
codigo = "0110 .... | dir:8 | dir:8"
semantica = "si Z: pc = dir"
ciclos = { tomado = 3, no_tomado = 2 }
  [[instruccion.operando]]
  nombre = "dir"
  tipo = "direccion"

# Memoria de programa.
[[region]]
nombre = "rom"
desde = 0x0000
hasta = 0x1FFF
acceso = "r"

[[region]]
nombre = "ram"
desde = 0x2000
hasta = 0xFFFF
"#;

fn doc() -> String {
    let m = lorenz_arch::cargar(TEXTO).unwrap();
    documentar(&m, &Comentarios::leer(TEXTO))
}

#[test]
fn es_determinista() {
    assert_eq!(doc(), doc());
}

#[test]
fn lleva_el_titulo_y_el_comentario_general() {
    let d = doc();
    assert!(d.starts_with("# demo\n"));
    assert!(d.contains("Maquina de prueba"));
    assert!(d.contains("Sirve para comprobar la documentacion."));
}

#[test]
fn la_tabla_de_codigos_sale_ordenada_por_codigo_de_operacion() {
    let d = doc();
    let tabla = &d[d.find("## Codigos de operacion").unwrap()..];
    let add = tabla.find("`ADD`").unwrap();
    let jz = tabla.find("`JZ`").unwrap();
    let hlt = tabla.find("`HLT`").unwrap();
    // 0011 (ADD) < 0110 (JZ) < 1111 (HLT)
    assert!(add < jz && jz < hlt);
}

#[test]
fn la_tabla_de_codigos_no_rompe_la_celda_con_la_barra_de_palabra_extra() {
    assert!(doc().contains("`0110 .... \\| dir:8 \\| dir:8`"));
}

#[test]
fn el_diagrama_de_codificacion_nombra_los_campos_y_los_bits() {
    let d = doc();
    let add = &d[d.find("### ADD").unwrap()..d.find("### HLT").unwrap_or(d.len())];
    assert!(add.contains("palabra 1 (8 bits)"));
    assert!(add.contains("| 0 | 0 | 1 | 1 |"));
    assert!(add.contains("rd"));
    assert!(add.contains("rs"));
}

#[test]
fn una_instruccion_de_dos_palabras_tiene_dos_diagramas() {
    let d = doc();
    let jz = &d[d.find("### JZ").unwrap()..];
    assert!(jz.contains("palabra 1") && jz.contains("palabra 2"));
}

#[test]
fn el_mapa_de_memoria_lleva_rangos_y_permisos() {
    let d = doc();
    assert!(d.contains("| `rom` | 0x0000 | 0x1FFF | 8192 | solo lectura |"));
    assert!(d.contains("| `ram` | 0x2000 | 0xFFFF | 57344 | lectura y escritura |"));
}

#[test]
fn registros_pares_y_banderas_aparecen_con_sus_comentarios() {
    let d = doc();
    assert!(d.contains("| `A` | 8 | general | 0x0 | Acumulador principal. |"));
    assert!(d.contains("| `AB` | `A` | `B` | 16 |"));
    assert!(d.contains("| `Z` | 0 | Se enciende si el resultado es cero. |"));
    assert!(d.contains("| `C` | 1 | acarreo |"));
}

#[test]
fn el_coste_en_ciclos_distingue_los_saltos() {
    let d = doc();
    assert!(d.contains("| `JZ` | `JZ {dir}` | 3 si salta, 2 si no |"));
    assert!(d.contains("| `ADD` | `ADD {rd}, {rs}` | 2 | Z, C |"));
}

#[test]
fn el_comentario_de_una_tabla_llega_a_la_instruccion() {
    let d = doc();
    let add = &d[d.find("### ADD").unwrap()..];
    assert!(add.contains("Suma dos registros.\nDeja el resultado en el primero."));
    assert!(add.contains("Suma.\n"));
}

#[test]
fn un_comentario_separado_por_una_linea_en_blanco_no_se_atribuye() {
    let c = Comentarios::leer(
        "# suelto\n\n[maquina]\nnombre = \"x\"\n\n# otro\n\n[[registro]]\nnombre = \"A\"\n",
    );
    assert_eq!(c.de("registro", "A"), "");
}

#[test]
fn comentarios_de_region_y_de_elemento_inexistente() {
    let c = Comentarios::leer(TEXTO);
    assert_eq!(c.de("region", "rom"), "Memoria de programa.");
    assert_eq!(c.de("instruccion", "NOPE"), "");
}
