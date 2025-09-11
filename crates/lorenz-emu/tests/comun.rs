//! Utilidades compartidas por las pruebas del emulador.
#![allow(dead_code)]

use lorenz_arch::modelo::Maquina;

/// La maquina de ejemplo de 8 bits del repositorio.
pub fn ejemplo_8() -> Maquina {
    let texto = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    lorenz_arch::cargar(&texto).expect("ejemplo-8 debe cargar")
}

/// Una maquina minima con tres regiones: rom (r), ram (rw) y buzon (w).
pub fn con_tres_regiones() -> Maquina {
    let texto = r#"
[maquina]
nombre = "tres"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

[[instruccion]]
mnemonico = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[region]]
nombre = "rom"
desde = 0x0000
hasta = 0x00FF
acceso = "r"

[[region]]
nombre = "ram"
desde = 0x0100
hasta = 0x01FF
acceso = "rw"

[[region]]
nombre = "buzon"
desde = 0x0300
hasta = 0x030F
acceso = "w"
"#;
    lorenz_arch::cargar(texto).expect("la maquina de prueba debe cargar")
}

use lorenz_emu::{Decodificador, Entorno, Estado, Interprete, Memoria, Puertos, Resultado};

/// Todo lo necesario para ejecutar a mano una instruccion.
pub struct Banco {
    pub maquina: Maquina,
    pub estado: Estado,
    pub memoria: Memoria,
    pub puertos: Puertos,
}

/// Maquina de 8 bits con una unica instruccion `X` (codigo `0000 rd:2 rs:2`) cuya semantica y
/// lista `afecta` se dan como parametros. Tiene A, B, C, D, SP, PC y F con las banderas Z, N, C, V.
pub fn maquina_con_semantica(semantica: &str, afecta: &str) -> Maquina {
    let texto = format!(
        r#"
[maquina]
nombre = "banco"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[maquina.interrupciones]
vectores = {{ 0 = 0x0010 }}
habilitadas_al_arrancar = false

[flags]
Z = {{ bit = 0, descripcion = "cero" }}
C = {{ bit = 1, descripcion = "acarreo" }}
N = {{ bit = 2, descripcion = "negativo" }}
V = {{ bit = 3, descripcion = "desborde" }}

[[registro]]
nombre = "A"
bits = 8
rol = "general"
inicial = 0
[[registro]]
nombre = "B"
bits = 8
rol = "general"
inicial = 0
[[registro]]
nombre = "C"
bits = 8
rol = "general"
inicial = 0
[[registro]]
nombre = "D"
bits = 8
rol = "general"
inicial = 0
[[registro]]
nombre = "SP"
bits = 16
rol = "sp"
inicial = 0x00FF
[[registro]]
nombre = "PC"
bits = 16
rol = "pc"
[[registro]]
nombre = "F"
bits = 8
rol = "flags"

[[instruccion]]
mnemonico = "X"
sintaxis = "X {{rd}}, {{rs}}"
codigo = "0000 rd:2 rs:2"
semantica = "{semantica}"
afecta = {afecta}
ciclos = 1
descripcion = "instruccion de prueba"
  [[instruccion.operando]]
  nombre = "rd"
  tipo = "registro"
  de = ["A", "B", "C", "D"]
  [[instruccion.operando]]
  nombre = "rs"
  tipo = "registro"
  de = ["A", "B", "C", "D"]

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0x00FF
acceso = "rw"
[[region]]
nombre = "rom"
desde = 0x0100
hasta = 0x01FF
acceso = "r"
[[region]]
nombre = "buzon"
desde = 0x0200
hasta = 0x02FF
acceso = "w"

[[dispositivo]]
nombre = "teclado"
tipo = "teclado"
puertos = {{ datos = 0x01, estado = 0x02 }}
"#
    );
    lorenz_arch::cargar(&texto).unwrap_or_else(|e| panic!("la maquina de prueba no carga: {e:?}"))
}

impl Banco {
    pub fn nuevo(semantica: &str, afecta: &str) -> Self {
        let maquina = maquina_con_semantica(semantica, afecta);
        let estado = Estado::nuevo(&maquina);
        let memoria = Memoria::nueva(&maquina).unwrap();
        let puertos = Puertos::nuevo(&maquina);
        Banco {
            maquina,
            estado,
            memoria,
            puertos,
        }
    }

    /// Ejecuta `X rd, rs` con los registros dados, como si el PC ya apuntara a la siguiente.
    pub fn ejecutar(&mut self, rd: u64, rs: u64) -> Result<Resultado, lorenz_emu::ErrorEjecucion> {
        let palabra = (rd << 2) | rs;
        let dec = Decodificador::nuevo(&self.maquina).decodificar_palabras(0, &[palabra])?;
        self.estado.fijar_pc(1);
        let mut entorno = Entorno {
            estado: &mut self.estado,
            memoria: &mut self.memoria,
            puertos: &mut self.puertos,
        };
        Interprete::nuevo(&self.maquina).ejecutar(
            &self.maquina.instrucciones[dec.indice],
            &dec,
            &mut entorno,
        )
    }

    pub fn reg(&self, nombre: &str) -> u64 {
        self.estado.registro(nombre).unwrap()
    }

    pub fn fijar(&mut self, nombre: &str, valor: u64) {
        self.estado.fijar_registro(nombre, valor).unwrap();
    }
}

/// Maquina con un disco de 4 bloques de 4 palabras: puertos bloque 0x20, datos 0x21, estado 0x22.
pub fn con_disco() -> Maquina {
    let texto = "
[maquina]
nombre = \"disco\"
formato = \"0.1\"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[[registro]]
nombre = \"PC\"
bits = 16
rol = \"pc\"

[[instruccion]]
mnemonico = \"HLT\"
codigo = \"1111 ....\"
semantica = \"detener\"

[[region]]
nombre = \"ram\"
desde = 0x0000
hasta = 0x00FF
acceso = \"rw\"

[[dispositivo]]
nombre = \"disco\"
tipo = \"bloques\"
tamano_bloque = 4
cantidad_bloques = 4
puertos = { bloque = 0x20, datos = 0x21, estado = 0x22 }
";
    lorenz_arch::cargar(texto).expect("la maquina con disco debe cargar")
}

/// Maquina con un dispositivo generico `eco` (puertos entrada 0x30 y salida 0x31, linea 0) cuyo
/// script se da como parametro. Tiene `OUT` (`0001 ....`, escribe A en el puerto 0x30), `OUT2`
/// (`0010 ....`, escribe A en el 0x31), `ST0` y `ST3` (`0011 ....` y `0100 ....`, escriben A en las
/// posiciones 0 y 3 de la region `buzon`, mapeada al dispositivo) y `HLT`.
pub fn con_script(script: &str) -> Maquina {
    let texto = format!(
        "
[maquina]
nombre = \"scripts\"
formato = \"0.1\"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[maquina.interrupciones]
vectores = {{ 0 = 0x0010 }}
habilitadas_al_arrancar = false

[[registro]]
nombre = \"A\"
bits = 8
rol = \"general\"
inicial = 0
[[registro]]
nombre = \"PC\"
bits = 16
rol = \"pc\"

[[instruccion]]
mnemonico = \"OUT\"
codigo = \"0001 ....\"
semantica = \"out[48] = A\"
ciclos = 1
[[instruccion]]
mnemonico = \"OUT2\"
codigo = \"0010 ....\"
semantica = \"out[49] = A\"
ciclos = 1
[[instruccion]]
mnemonico = \"ST0\"
codigo = \"0011 ....\"
semantica = \"mem[256] = A\"
ciclos = 1
[[instruccion]]
mnemonico = \"ST3\"
codigo = \"0100 ....\"
semantica = \"mem[259] = A\"
ciclos = 1
[[instruccion]]
mnemonico = \"HLT\"
codigo = \"1111 ....\"
semantica = \"detener\"

[[region]]
nombre = \"ram\"
desde = 0x0000
hasta = 0x00FF
acceso = \"rw\"

[[region]]
nombre = \"buzon\"
desde = 0x0100
hasta = 0x010F
acceso = \"rw\"
dispositivo = \"eco\"

[[dispositivo]]
nombre = \"eco\"
tipo = \"generico\"
puertos = {{ entrada = 0x30, salida = 0x31 }}
interrupcion = 0
script = \"{script}\"
"
    );
    lorenz_arch::cargar(&texto).unwrap_or_else(|e| panic!("la maquina con script no carga: {e:?}"))
}
