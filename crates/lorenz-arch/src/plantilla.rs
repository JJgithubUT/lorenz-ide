//! Plantillas de sintaxis: el campo `sintaxis` de cada instruccion, como `"LD {rd}, [{dir}]"`.
//!
//! Una plantilla es una secuencia de texto literal y de huecos `{nombre}`. El mismo objeto sirve
//! para dos cosas: reconocer una linea de ensamblador y recuperar el texto de cada hueco
//! (ensamblador), y rellenar los huecos con los operandos decodificados (desensamblador).
//! RF-ENS-002, RF-ENS-022.

use std::fmt;

/// Un trozo de una plantilla.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trozo {
    /// Texto que debe aparecer tal cual (sin espacios: estos son opcionales al leer).
    Literal(String),
    /// Un operando, por su nombre.
    Hueco(String),
}

/// Por que no se pudo construir una plantilla.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPlantilla {
    Vacia,
    LlaveSinCerrar,
    LlaveSinAbrir,
    HuecoVacio,
    /// Dos huecos pegados no se pueden separar al leer.
    HuecosContiguos(String, String),
}

impl fmt::Display for ErrorPlantilla {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorPlantilla::Vacia => write!(f, "la sintaxis esta vacia"),
            ErrorPlantilla::LlaveSinCerrar => write!(f, "falta la llave de cierre `}}`"),
            ErrorPlantilla::LlaveSinAbrir => write!(f, "hay una llave de cierre `}}` sin abrir"),
            ErrorPlantilla::HuecoVacio => write!(f, "hay un hueco `{{}}` sin nombre"),
            ErrorPlantilla::HuecosContiguos(a, b) => write!(
                f,
                "los operandos `{a}` y `{b}` estan pegados: pon entre ellos un separador"
            ),
        }
    }
}

impl std::error::Error for ErrorPlantilla {}

/// Una sintaxis ya analizada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plantilla {
    mnemonico: String,
    trozos: Vec<Trozo>,
    /// Para cada trozo, si en la sintaxis original habia un espacio justo antes. Permite escribir
    /// la instruccion con el mismo aspecto que declaro el usuario.
    espacio_antes: Vec<bool>,
}

impl Plantilla {
    /// Analiza el texto de una sintaxis. La primera palabra es el mnemonico.
    pub fn analizar(texto: &str) -> Result<Self, ErrorPlantilla> {
        let texto = texto.trim();
        if texto.is_empty() {
            return Err(ErrorPlantilla::Vacia);
        }
        let fin_mnemonico = texto
            .find(|c: char| c.is_whitespace())
            .unwrap_or(texto.len());
        let mnemonico = texto[..fin_mnemonico].to_string();
        if mnemonico.contains('{') || mnemonico.contains('}') {
            return Err(ErrorPlantilla::HuecoVacio);
        }
        let resto = &texto[fin_mnemonico..];
        let mut trozos: Vec<Trozo> = Vec::new();
        let mut espacio_antes: Vec<bool> = Vec::new();
        let mut literal = String::new();
        let mut espacio_literal = false;
        let mut espacio = false;
        let mut chars = resto.chars();
        while let Some(c) = chars.next() {
            match c {
                '{' => {
                    if !literal.is_empty() {
                        trozos.push(Trozo::Literal(std::mem::take(&mut literal)));
                        espacio_antes.push(espacio_literal);
                    }
                    let mut nombre = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(otro) => nombre.push(otro),
                            None => return Err(ErrorPlantilla::LlaveSinCerrar),
                        }
                    }
                    let nombre = nombre.trim().to_string();
                    if nombre.is_empty() {
                        return Err(ErrorPlantilla::HuecoVacio);
                    }
                    if let Some(Trozo::Hueco(previo)) = trozos.last() {
                        return Err(ErrorPlantilla::HuecosContiguos(previo.clone(), nombre));
                    }
                    trozos.push(Trozo::Hueco(nombre));
                    espacio_antes.push(espacio);
                    espacio = false;
                }
                '}' => return Err(ErrorPlantilla::LlaveSinAbrir),
                c if c.is_whitespace() => {
                    // Un espacio separa trozos pero no es parte del literal.
                    if !literal.is_empty() {
                        trozos.push(Trozo::Literal(std::mem::take(&mut literal)));
                        espacio_antes.push(espacio_literal);
                    }
                    espacio = true;
                }
                c => {
                    if literal.is_empty() {
                        espacio_literal = espacio;
                        espacio = false;
                    }
                    literal.push(c);
                }
            }
        }
        if !literal.is_empty() {
            trozos.push(Trozo::Literal(literal));
            espacio_antes.push(espacio_literal);
        }
        Ok(Plantilla {
            mnemonico,
            trozos,
            espacio_antes,
        })
    }

    pub fn mnemonico(&self) -> &str {
        &self.mnemonico
    }

    pub fn trozos(&self) -> &[Trozo] {
        &self.trozos
    }

    /// Nombres de los huecos, en el orden en que se escriben.
    pub fn huecos(&self) -> Vec<&str> {
        self.trozos
            .iter()
            .filter_map(|t| match t {
                Trozo::Hueco(n) => Some(n.as_str()),
                Trozo::Literal(_) => None,
            })
            .collect()
    }

    /// Reconoce los operandos de una linea. `texto` es lo que sigue al mnemonico.
    ///
    /// Devuelve el texto de cada hueco, recortado, en el orden de la plantilla. `None` si la linea
    /// no encaja. Las mayusculas no importan en los literales.
    pub fn reconocer(&self, texto: &str) -> Option<Vec<(String, String)>> {
        self.reconocer_con_posicion(texto)
            .map(|v| v.into_iter().map(|(n, t, _)| (n, t)).collect())
    }

    /// Como [`Plantilla::reconocer`], pero tambien devuelve en que byte de `texto` empieza cada
    /// captura: es lo que permite senalar la columna exacta de un operando en un error.
    pub fn reconocer_con_posicion(&self, texto: &str) -> Option<Vec<(String, String, usize)>> {
        let mut resto = texto;
        let mut capturas = Vec::new();
        let mut i = 0;
        while i < self.trozos.len() {
            match &self.trozos[i] {
                Trozo::Literal(lit) => {
                    resto = resto.trim_start();
                    let n = lit.chars().count();
                    let cabeza: String = resto.chars().take(n).collect();
                    if !cabeza.eq_ignore_ascii_case(lit) {
                        return None;
                    }
                    resto = &resto[cabeza.len()..];
                }
                Trozo::Hueco(nombre) => {
                    let hasta = match self.trozos.get(i + 1) {
                        Some(Trozo::Literal(siguiente)) => buscar_en_nivel_cero(resto, siguiente)?,
                        _ => resto.len(),
                    };
                    let bruto = &resto[..hasta];
                    let captura = bruto.trim();
                    if captura.is_empty() {
                        return None;
                    }
                    let inicio =
                        texto.len() - resto.len() + (bruto.len() - bruto.trim_start().len());
                    capturas.push((nombre.clone(), captura.to_string(), inicio));
                    resto = &resto[hasta..];
                }
            }
            i += 1;
        }
        if resto.trim().is_empty() {
            Some(capturas)
        } else {
            None
        }
    }

    /// Escribe la instruccion rellenando cada hueco con el texto dado por `valor`. Respeta los
    /// espacios de la sintaxis declarada: `LD {rd}, [{dir}]` produce `LD A, [0x2000]`.
    pub fn componer(&self, valor: impl Fn(&str) -> String) -> String {
        let mut salida = self.mnemonico.clone();
        for (trozo, espacio) in self.trozos.iter().zip(&self.espacio_antes) {
            if *espacio {
                salida.push(' ');
            }
            match trozo {
                Trozo::Hueco(n) => salida.push_str(&valor(n)),
                Trozo::Literal(l) => salida.push_str(l),
            }
        }
        salida
    }
}

/// Posicion de la primera aparicion de `aguja` fuera de parentesis, corchetes y comillas.
fn buscar_en_nivel_cero(texto: &str, aguja: &str) -> Option<usize> {
    let mut nivel = 0i32;
    let mut en_comilla = false;
    for (pos, c) in texto.char_indices() {
        if en_comilla {
            if c == '\'' {
                en_comilla = false;
            }
            continue;
        }
        if nivel == 0 && texto[pos..].len() >= aguja.len() {
            let candidato = &texto[pos..pos + aguja.len()];
            if texto.is_char_boundary(pos + aguja.len()) && candidato.eq_ignore_ascii_case(aguja) {
                return Some(pos);
            }
        }
        match c {
            '\'' => en_comilla = true,
            '(' => nivel += 1,
            ')' => nivel -= 1,
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn analiza_mnemonico_y_huecos() {
        let p = Plantilla::analizar("LD {rd}, [{dir}]").unwrap();
        assert_eq!(p.mnemonico(), "LD");
        assert_eq!(p.huecos(), vec!["rd", "dir"]);
    }

    #[test]
    fn sin_operandos_no_tiene_trozos() {
        let p = Plantilla::analizar("HLT").unwrap();
        assert!(p.trozos().is_empty());
        assert_eq!(p.reconocer(""), Some(vec![]));
        assert_eq!(p.reconocer("A"), None);
    }

    #[test]
    fn rechaza_plantillas_mal_formadas() {
        assert_eq!(Plantilla::analizar("  "), Err(ErrorPlantilla::Vacia));
        assert_eq!(
            Plantilla::analizar("MOV {rd"),
            Err(ErrorPlantilla::LlaveSinCerrar)
        );
        assert_eq!(
            Plantilla::analizar("MOV rd}"),
            Err(ErrorPlantilla::LlaveSinAbrir)
        );
        assert_eq!(
            Plantilla::analizar("MOV {}"),
            Err(ErrorPlantilla::HuecoVacio)
        );
        assert!(matches!(
            Plantilla::analizar("MOV {a}{b}"),
            Err(ErrorPlantilla::HuecosContiguos(_, _))
        ));
    }

    #[test]
    fn reconoce_ignorando_espacios_y_mayusculas() {
        let p = Plantilla::analizar("LD {rd}, [{dir}]").unwrap();
        let c = p.reconocer(" a ,[ 0x20 + 1 ] ").unwrap();
        assert_eq!(c[0], ("rd".to_string(), "a".to_string()));
        assert_eq!(c[1], ("dir".to_string(), "0x20 + 1".to_string()));
    }

    #[test]
    fn la_coma_dentro_de_parentesis_no_separa() {
        let p = Plantilla::analizar("OP {a}, {b}").unwrap();
        let c = p.reconocer("f(1,2), 3").unwrap();
        assert_eq!(c[0].1, "f(1,2)");
        assert_eq!(c[1].1, "3");
    }

    #[test]
    fn una_coma_dentro_de_un_caracter_no_separa() {
        let p = Plantilla::analizar("OP {a}, {b}").unwrap();
        let c = p.reconocer("',', 3").unwrap();
        assert_eq!(c[0].1, "','");
    }

    #[test]
    fn no_reconoce_si_falta_o_sobra_texto() {
        let p = Plantilla::analizar("ADD {rd}, {rs}").unwrap();
        assert!(p.reconocer("A").is_none());
        assert!(p.reconocer("A B").is_none());
        assert!(p.reconocer(", B").is_none());
    }

    #[test]
    fn la_posicion_de_cada_captura_distingue_textos_repetidos() {
        let p = Plantilla::analizar("OP {a}, {b}").unwrap();
        let c = p.reconocer_con_posicion(" A, A").unwrap();
        assert_eq!(c[0], ("a".to_string(), "A".to_string(), 1));
        assert_eq!(c[1], ("b".to_string(), "A".to_string(), 4));
    }

    #[test]
    fn compone_el_texto_de_la_instruccion() {
        let p = Plantilla::analizar("LD {rd}, [{dir}]").unwrap();
        let texto = p.componer(|n| {
            if n == "rd" {
                "A".into()
            } else {
                "0x2000".into()
            }
        });
        assert_eq!(texto, "LD A, [0x2000]");
        let q = Plantilla::analizar("HLT").unwrap();
        assert_eq!(q.componer(|_| String::new()), "HLT");
        let r = Plantilla::analizar("JZ {dir}").unwrap();
        assert_eq!(r.componer(|_| "0x10".into()), "JZ 0x10");
    }
}
