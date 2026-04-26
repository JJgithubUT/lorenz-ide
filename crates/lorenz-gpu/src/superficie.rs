//! La superficie de dibujo de la unidad grafica: color y, si se pide, profundidad (RF-GPU-002).

/// Bits por pixel del framebuffer. El color se guarda siempre como 24 bits `0xRRGGBB`, pero al
/// escribirlo se redondea a lo que la profundidad puede representar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfundidadColor {
    /// 3 bits de rojo, 3 de verde y 2 de azul.
    Bits8,
    /// 5 bits de rojo, 6 de verde y 5 de azul.
    Bits16,
    /// 8 bits por canal.
    Bits24,
}

impl ProfundidadColor {
    pub fn desde_bits(bits: u32) -> Option<Self> {
        match bits {
            8 => Some(Self::Bits8),
            16 => Some(Self::Bits16),
            24 => Some(Self::Bits24),
            _ => None,
        }
    }

    pub fn bits(self) -> u32 {
        match self {
            Self::Bits8 => 8,
            Self::Bits16 => 16,
            Self::Bits24 => 24,
        }
    }

    /// Redondea un color `0xRRGGBB` a los bits que esta profundidad conserva.
    pub fn cuantizar(self, rgb: u32) -> u32 {
        let (r, g, b) = descomponer(rgb);
        let (r, g, b) = match self {
            Self::Bits24 => (r, g, b),
            Self::Bits16 => (ampliar(r >> 3, 5), ampliar(g >> 2, 6), ampliar(b >> 3, 5)),
            Self::Bits8 => (ampliar(r >> 5, 3), ampliar(g >> 5, 3), ampliar(b >> 6, 2)),
        };
        componer(r, g, b)
    }
}

/// Lleva un valor de `bits` bits al rango completo de 8 bits replicando sus bits altos.
fn ampliar(valor: u32, bits: u32) -> u32 {
    let mut v = valor << (8 - bits);
    let mut restante = bits;
    while restante < 8 {
        v |= v >> restante;
        restante *= 2;
    }
    v & 0xFF
}

pub fn descomponer(rgb: u32) -> (u32, u32, u32) {
    ((rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF)
}

pub fn componer(r: u32, g: u32, b: u32) -> u32 {
    (r.min(255) << 16) | (g.min(255) << 8) | b.min(255)
}

/// Resolucion y formato de una superficie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigSuperficie {
    pub ancho: u32,
    pub alto: u32,
    pub color: ProfundidadColor,
    /// Si hay buffer de profundidad (RF-GPU-009).
    pub profundidad: bool,
}

/// Profundidad de un pixel que todavia no se dibujo: lo mas lejano posible.
pub const PROFUNDIDAD_LEJANA: u32 = u32::MAX;

/// El framebuffer y, si esta activado, su buffer de profundidad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Superficie {
    config: ConfigSuperficie,
    pixeles: Vec<u32>,
    z: Vec<u32>,
}

impl Superficie {
    pub fn nueva(config: ConfigSuperficie) -> Self {
        let n = (config.ancho as usize) * (config.alto as usize);
        Self {
            config,
            pixeles: vec![0; n],
            z: if config.profundidad {
                vec![PROFUNDIDAD_LEJANA; n]
            } else {
                Vec::new()
            },
        }
    }

    pub fn config(&self) -> ConfigSuperficie {
        self.config
    }

    pub fn ancho(&self) -> u32 {
        self.config.ancho
    }

    pub fn alto(&self) -> u32 {
        self.config.alto
    }

    fn indice(&self, x: i64, y: i64) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.config.ancho as i64 || y >= self.config.alto as i64 {
            return None;
        }
        Some(y as usize * self.config.ancho as usize + x as usize)
    }

    /// Color `0xRRGGBB` del pixel, o `None` si cae fuera.
    pub fn pixel(&self, x: i64, y: i64) -> Option<u32> {
        self.indice(x, y).map(|i| self.pixeles[i])
    }

    /// Profundidad guardada en el pixel; `None` si cae fuera o no hay buffer de profundidad.
    pub fn profundidad_en(&self, x: i64, y: i64) -> Option<u32> {
        let i = self.indice(x, y)?;
        self.z.get(i).copied()
    }

    /// Escribe un pixel. Con `z` y buffer de profundidad, solo si esta mas cerca que lo guardado.
    /// Devuelve si el pixel se escribio.
    pub fn poner(&mut self, x: i64, y: i64, rgb: u32, z: Option<u32>) -> bool {
        let Some(i) = self.indice(x, y) else {
            return false;
        };
        if let (Some(z), false) = (z, self.z.is_empty()) {
            if z >= self.z[i] {
                return false;
            }
            self.z[i] = z;
        }
        self.pixeles[i] = self.config.color.cuantizar(rgb);
        true
    }

    /// Borra el color y la profundidad.
    pub fn limpiar(&mut self, rgb: u32) {
        let c = self.config.color.cuantizar(rgb);
        self.pixeles.iter_mut().for_each(|p| *p = c);
        self.z.iter_mut().for_each(|z| *z = PROFUNDIDAD_LEJANA);
    }

    pub fn pixeles(&self) -> &[u32] {
        &self.pixeles
    }
}
