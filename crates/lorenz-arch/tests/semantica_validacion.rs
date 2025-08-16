//! Pruebas de la validacion de la semantica dentro del cargador (RF-ARQ-022).

use lorenz_arch::semantica::{Destino, Expr, OpBinario, Sentencia};
use lorenz_arch::{cargar, ErrorArquitectura, Regla};

fn base(semantica: &str) -> String {
    format!(
        r#"
[maquina]
nombre = "prueba"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[flags]
Z = {{ bit = 0, descripcion = "cero" }}

[[registro]]
nombre = "A"
bits = 8
rol = "general"

[[registro]]
nombre = "K"
bits = 8
rol = "fijo"

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[registro]]
nombre = "SP"
bits = 16
rol = "sp"

[[registro]]
nombre = "F"
bits = 8
rol = "flags"

[[instruccion]]
mnemonico = "OP"
codigo = "0001 rd:2 imm:2"
semantica = "{semantica}"
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "A", "A", "A"]
  [[instruccion.operando]]
  nombre = "imm"
  tipo = "inmediato"
  rango = [0, 3]

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
acceso = "rw"
"#
    )
}

fn fallos(texto: &str) -> Vec<(Regla, String)> {
    match cargar(texto) {
        Ok(_) => Vec::new(),
        Err(ErrorArquitectura::Validacion(f)) => {
            f.into_iter().map(|x| (x.regla, x.detalle)).collect()
        }
        Err(e) => panic!("{e}"),
    }
}

fn hay_semantica_invalida(texto: &str) -> bool {
    fallos(texto)
        .iter()
        .any(|(r, _)| *r == Regla::SemanticaInvalida)
}

#[test]
fn una_semantica_valida_se_acepta() {
    assert!(cargar(&base("rd = rd + imm")).is_ok());
}

#[test]
fn una_semantica_vacia_se_acepta() {
    assert!(cargar(&base("")).is_ok());
}

#[test]
fn el_arbol_queda_guardado_en_la_instruccion() {
    let m = cargar(&base("rd = rd + imm")).unwrap();
    let p = &m.instruccion("OP").unwrap().programa;
    assert_eq!(p.sentencias.len(), 1);
    assert_eq!(
        p.sentencias[0],
        Sentencia::Asignar(
            Destino::Nombre("rd".into()),
            Expr::Binario(
                OpBinario::Suma,
                Box::new(Expr::Nombre("rd".into())),
                Box::new(Expr::Nombre("imm".into()))
            )
        )
    );
}

#[test]
fn el_texto_original_tambien_se_conserva() {
    let m = cargar(&base("rd = imm")).unwrap();
    assert_eq!(m.instruccion("OP").unwrap().semantica, "rd = imm");
}

#[test]
fn una_sintaxis_invalida_se_reporta_como_e_semant() {
    assert!(hay_semantica_invalida(&base("rd = ")));
}

#[test]
fn el_mensaje_nombra_la_instruccion_y_la_causa() {
    let f = fallos(&base("rd = "));
    let (_, detalle) = f
        .iter()
        .find(|(r, _)| *r == Regla::SemanticaInvalida)
        .unwrap();
    assert!(detalle.contains("'OP'"), "{detalle}");
    assert!(detalle.contains("se esperaba"), "{detalle}");
}

#[test]
fn citar_un_nombre_inexistente_se_rechaza() {
    assert!(hay_semantica_invalida(&base("rd = fantasma")));
}

#[test]
fn asignar_a_un_nombre_inexistente_se_rechaza() {
    assert!(hay_semantica_invalida(&base("fantasma = 1")));
}

#[test]
fn se_pueden_leer_registros_por_su_nombre() {
    assert!(cargar(&base("rd = A")).is_ok());
}

#[test]
fn se_pueden_asignar_registros_por_su_nombre() {
    assert!(cargar(&base("A = imm")).is_ok());
}

#[test]
fn se_pueden_usar_las_banderas() {
    assert!(cargar(&base("Z = rd == 0")).is_ok());
    assert!(cargar(&base("si Z: rd = 1")).is_ok());
}

#[test]
fn los_alias_pc_y_sp_se_aceptan() {
    assert!(cargar(&base("pc = pc + 1")).is_ok());
    assert!(cargar(&base("sp = sp - 1; mem[sp] = rd")).is_ok());
}

#[test]
fn el_alias_sp_exige_un_registro_con_ese_rol() {
    let texto = base("sp = 1").replace(
        r#"[[registro]]
nombre = "SP"
bits = 16
rol = "sp"
"#,
        "",
    );
    assert!(hay_semantica_invalida(&texto));
}

#[test]
fn asignar_a_un_operando_inmediato_se_rechaza() {
    assert!(hay_semantica_invalida(&base("imm = 1")));
}

#[test]
fn asignar_a_un_registro_fijo_se_rechaza() {
    assert!(hay_semantica_invalida(&base("K = 1")));
}

#[test]
fn leer_un_registro_fijo_se_acepta() {
    assert!(cargar(&base("rd = K")).is_ok());
}

#[test]
fn los_nombres_dentro_de_un_indice_de_memoria_tambien_se_comprueban() {
    assert!(hay_semantica_invalida(&base("mem[fantasma] = rd")));
    assert!(hay_semantica_invalida(&base("rd = mem[fantasma]")));
}

#[test]
fn los_nombres_dentro_de_un_condicional_tambien_se_comprueban() {
    assert!(hay_semantica_invalida(&base("si fantasma: rd = 1")));
    assert!(hay_semantica_invalida(&base("si Z: rd = fantasma")));
}

#[test]
fn los_nombres_dentro_de_un_puerto_tambien_se_comprueban() {
    assert!(hay_semantica_invalida(&base("out[fantasma] = rd")));
}

#[test]
fn un_error_de_semantica_no_impide_ver_los_demas_fallos() {
    let texto = base("rd = ").replace("bits_palabra = 8", "bits_palabra = 3");
    let f = fallos(&texto);
    assert!(f.iter().any(|(r, _)| *r == Regla::SemanticaInvalida));
    assert!(f.iter().any(|(r, _)| *r == Regla::ValorFueraDeRango));
}

#[test]
fn e_semant_cita_el_requisito_correcto() {
    assert_eq!(Regla::SemanticaInvalida.codigo(), "E-SEMANT");
    assert_eq!(Regla::SemanticaInvalida.requisito(), "RF-ARQ-022");
}

#[test]
fn la_maquina_de_ejemplo_tiene_semantica_analizada_en_todas_sus_instrucciones() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    let con_programa = m
        .instrucciones
        .iter()
        .filter(|i| !i.programa.sentencias.is_empty())
        .count();
    // NOP no hace nada; las otras siete tienen cuerpo
    assert_eq!(con_programa, 7);
    assert!(m.instruccion("NOP").unwrap().programa.sentencias.is_empty());
}

#[test]
fn jz_de_ejemplo_se_analiza_como_condicional() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    let p = &m.instruccion("JZ").unwrap().programa;
    assert!(matches!(p.sentencias[0], Sentencia::Si(_, _)));
}

#[test]
fn hlt_de_ejemplo_puede_detener() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    assert!(m.instruccion("HLT").unwrap().programa.puede_detener());
    assert!(!m.instruccion("ADD").unwrap().programa.puede_detener());
}

#[test]
fn nombres_leidos_y_escritos_de_add() {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = cargar(&texto).unwrap();
    let p = &m.instruccion("ADD").unwrap().programa;
    assert_eq!(p.nombres_leidos(), vec!["rd".to_string(), "rs".to_string()]);
    assert_eq!(p.nombres_escritos(), vec!["rd".to_string()]);
}

fn con_interrupciones(semantica: &str) -> String {
    base(semantica).replace(
        "[flags]",
        "[maquina.interrupciones]\nvectores = { 0 = 0x0010 }\nhabilitadas_al_arrancar = false\n\n[flags]",
    )
}

#[test]
fn el_alias_ie_se_acepta_si_la_maquina_tiene_interrupciones() {
    assert!(cargar(&con_interrupciones("ie = 1")).is_ok());
    assert!(cargar(&con_interrupciones("si ie: ie = 0")).is_ok());
}

#[test]
fn el_alias_ie_se_rechaza_si_la_maquina_no_tiene_interrupciones() {
    assert!(hay_semantica_invalida(&base("ie = 1")));
    assert!(hay_semantica_invalida(&base("si ie: rd = 1")));
}
