//! JSON minimo: analizador con limite de profundidad y serializador.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Nulo,
    Bool(bool),
    Num(f64),
    Texto(String),
    Lista(Vec<Json>),
    Objeto(BTreeMap<String, Json>),
}

const PROFUNDIDAD_MAX: usize = 64;

impl Json {
    pub fn objeto<const N: usize>(pares: [(&str, Json); N]) -> Json {
        Json::Objeto(pares.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn texto(t: impl Into<String>) -> Json {
        Json::Texto(t.into())
    }

    pub fn get(&self, clave: &str) -> Option<&Json> {
        match self {
            Json::Objeto(m) => m.get(clave),
            _ => None,
        }
    }

    pub fn como_texto(&self) -> Option<&str> {
        match self {
            Json::Texto(t) => Some(t),
            _ => None,
        }
    }

    pub fn como_num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// Entero sin parte decimal dentro del rango de `i64`.
    pub fn como_entero(&self) -> Option<i64> {
        let n = self.como_num()?;
        (n.fract() == 0.0 && n.abs() < 9.0e15).then_some(n as i64)
    }

    pub fn como_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn como_lista(&self) -> Option<&[Json]> {
        match self {
            Json::Lista(l) => Some(l),
            _ => None,
        }
    }

    pub fn analizar(texto: &str) -> Result<Json, String> {
        let mut a = Analizador {
            b: texto.as_bytes(),
            i: 0,
        };
        a.espacios();
        let v = a.valor(0)?;
        a.espacios();
        if a.i != a.b.len() {
            return Err(format!("sobra texto en la posicion {}", a.i));
        }
        Ok(v)
    }

    pub fn serializar(&self) -> String {
        let mut s = String::new();
        self.escribir(&mut s);
        s
    }

    fn escribir(&self, s: &mut String) {
        match self {
            Json::Nulo => s.push_str("null"),
            Json::Bool(b) => s.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) if n.is_finite() => {
                if n.fract() == 0.0 && n.abs() < 9.0e15 {
                    s.push_str(&format!("{}", *n as i64));
                } else {
                    s.push_str(&format!("{n}"));
                }
            }
            Json::Num(_) => s.push_str("null"),
            Json::Texto(t) => escribir_texto(t, s),
            Json::Lista(l) => {
                s.push('[');
                for (i, v) in l.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    v.escribir(s);
                }
                s.push(']');
            }
            Json::Objeto(m) => {
                s.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    escribir_texto(k, s);
                    s.push(':');
                    v.escribir(s);
                }
                s.push('}');
            }
        }
    }
}

impl From<&str> for Json {
    fn from(t: &str) -> Json {
        Json::Texto(t.into())
    }
}
impl From<String> for Json {
    fn from(t: String) -> Json {
        Json::Texto(t)
    }
}
impl From<bool> for Json {
    fn from(b: bool) -> Json {
        Json::Bool(b)
    }
}
impl From<u64> for Json {
    fn from(n: u64) -> Json {
        Json::Num(n as f64)
    }
}
impl From<i64> for Json {
    fn from(n: i64) -> Json {
        Json::Num(n as f64)
    }
}
impl From<usize> for Json {
    fn from(n: usize) -> Json {
        Json::Num(n as f64)
    }
}
impl From<Vec<Json>> for Json {
    fn from(l: Vec<Json>) -> Json {
        Json::Lista(l)
    }
}

fn escribir_texto(t: &str, s: &mut String) {
    s.push('"');
    for c in t.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            // `<` escapado: el JSON puede acabar dentro de un <script>.
            '<' => s.push_str("\\u003c"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                s.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => s.push(c),
        }
    }
    s.push('"');
}

struct Analizador<'a> {
    b: &'a [u8],
    i: usize,
}

impl Analizador<'_> {
    fn espacios(&mut self) {
        while matches!(self.b.get(self.i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn error<T>(&self, m: &str) -> Result<T, String> {
        Err(format!("{m} en la posicion {}", self.i))
    }

    fn literal(&mut self, t: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(t.as_bytes()) {
            self.i += t.len();
            Ok(v)
        } else {
            self.error("valor invalido")
        }
    }

    fn valor(&mut self, prof: usize) -> Result<Json, String> {
        if prof > PROFUNDIDAD_MAX {
            return self.error("anidamiento excesivo");
        }
        match self.b.get(self.i) {
            None => self.error("fin inesperado"),
            Some(b'n') => self.literal("null", Json::Nulo),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'"') => Ok(Json::Texto(self.texto()?)),
            Some(b'[') => {
                self.i += 1;
                let mut l = Vec::new();
                self.espacios();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Lista(l));
                }
                loop {
                    self.espacios();
                    l.push(self.valor(prof + 1)?);
                    self.espacios();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Lista(l));
                        }
                        _ => return self.error("se esperaba `,` o `]`"),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.espacios();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Objeto(m));
                }
                loop {
                    self.espacios();
                    if self.b.get(self.i) != Some(&b'"') {
                        return self.error("se esperaba una clave");
                    }
                    let k = self.texto()?;
                    self.espacios();
                    if self.b.get(self.i) != Some(&b':') {
                        return self.error("se esperaba `:`");
                    }
                    self.i += 1;
                    self.espacios();
                    let v = self.valor(prof + 1)?;
                    m.insert(k, v);
                    self.espacios();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Objeto(m));
                        }
                        _ => return self.error("se esperaba `,` o `}`"),
                    }
                }
            }
            Some(b'-' | b'0'..=b'9') => self.numero(),
            Some(_) => self.error("caracter inesperado"),
        }
    }

    fn numero(&mut self) -> Result<Json, String> {
        let ini = self.i;
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let digitos = |a: &mut Self| {
            let p = a.i;
            while matches!(a.b.get(a.i), Some(b'0'..=b'9')) {
                a.i += 1;
            }
            a.i > p
        };
        if !digitos(self) {
            return self.error("numero invalido");
        }
        if self.b.get(self.i) == Some(&b'.') {
            self.i += 1;
            if !digitos(self) {
                return self.error("numero invalido");
            }
        }
        if matches!(self.b.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !digitos(self) {
                return self.error("numero invalido");
            }
        }
        let t = std::str::from_utf8(&self.b[ini..self.i]).map_err(|e| e.to_string())?;
        t.parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .map(Json::Num)
            .ok_or_else(|| format!("numero fuera de rango en la posicion {ini}"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let h = self
            .b
            .get(self.i..self.i + 4)
            .and_then(|x| std::str::from_utf8(x).ok())
            .and_then(|x| u32::from_str_radix(x, 16).ok());
        match h {
            Some(v) => {
                self.i += 4;
                Ok(v)
            }
            None => self.error("escape \\u invalido"),
        }
    }

    fn texto(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut o: Vec<u8> = Vec::new();
        loop {
            match self.b.get(self.i) {
                None => return self.error("texto sin cerrar"),
                Some(b'"') => {
                    self.i += 1;
                    return String::from_utf8(o).map_err(|_| "texto que no es UTF-8".to_string());
                }
                Some(c) if *c < 0x20 => return self.error("caracter de control en un texto"),
                Some(b'\\') => {
                    self.i += 1;
                    let e = self.b.get(self.i).copied();
                    self.i += 1;
                    let c = match e {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'u') => {
                            let a = self.hex4()?;
                            if (0xD800..0xDC00).contains(&a) {
                                if self.b.get(self.i..self.i + 2) != Some(b"\\u") {
                                    return self.error("sustituto sin pareja");
                                }
                                self.i += 2;
                                let b = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&b) {
                                    return self.error("sustituto sin pareja");
                                }
                                char::from_u32(0x10000 + ((a - 0xD800) << 10) + (b - 0xDC00))
                                    .unwrap_or('\u{fffd}')
                            } else if (0xDC00..0xE000).contains(&a) {
                                return self.error("sustituto sin pareja");
                            } else {
                                char::from_u32(a).unwrap_or('\u{fffd}')
                            }
                        }
                        _ => return self.error("escape invalido"),
                    };
                    let mut tmp = [0u8; 4];
                    o.extend_from_slice(c.encode_utf8(&mut tmp).as_bytes());
                }
                Some(c) => {
                    o.push(*c);
                    self.i += 1;
                }
            }
        }
    }
}
