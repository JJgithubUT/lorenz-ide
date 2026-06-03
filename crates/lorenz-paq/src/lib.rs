//! Paquetes de Lorenz (fase 6, ecosistema): empaquetar una arquitectura, un dispositivo, una
//! biblioteca o un proyecto, verificar su integridad e instalarlo con sus dependencias.

pub mod almacen;
pub mod descarga;
pub mod paquete;
pub mod sha256;
pub mod version;

pub use almacen::{Almacen, Informe};
pub use paquete::{ErrorPaquete, Manifiesto, Paquete, Tipo, FORMATO_SOPORTADO};
pub use sha256::{sha256, sha256_hex};
pub use version::{Requisito, Version};
