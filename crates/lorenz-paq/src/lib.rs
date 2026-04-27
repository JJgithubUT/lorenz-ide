//! Paquetes de Lorenz (fase 6, ecosistema): empaquetar una arquitectura, un dispositivo, una
//! biblioteca o un proyecto, verificar su integridad e instalarlo con sus dependencias.

pub mod sha256;
pub mod version;

pub use sha256::{sha256, sha256_hex};
pub use version::{Requisito, Version};
