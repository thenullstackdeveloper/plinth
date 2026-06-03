//! Errores del dominio. Hablan el lenguaje del producto, nunca el de la
//! infraestructura (sin rutas de fichero, sin tipos de la capa de input, etc.).

use crate::action::ActionId;

/// Todo lo que puede salir mal dentro del núcleo agnóstico al juego.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// Una cadena de acción no pertenece ni al catálogo de juego ni al
    /// conjunto cerrado de acciones de app. No puede construirse un `Action`.
    #[error("acción desconocida: `{0}` (no está en el catálogo)")]
    UnknownAction(String),

    /// La acción de juego existe en el catálogo pero el usuario no le ha
    /// asignado ninguna tecla. Distinto de `UnknownAction`.
    #[error("la acción `{0}` no tiene ninguna tecla asignada")]
    ActionNotBound(ActionId),

    /// Intento de ir a una página fuera de rango.
    #[error("página fuera de rango: pedida {requested}, máximo {max}")]
    PageOutOfRange { requested: usize, max: usize },

    /// Navegación rechazada porque el candado está activo.
    #[error("el candado está activo: navegación bloqueada")]
    LockEngaged,

    /// Una plantilla sin páginas no es representable.
    #[error("la plantilla no tiene páginas")]
    EmptyTemplate,

    /// Dos componentes comparten identificador dentro de la plantilla.
    #[error("identificador de componente duplicado: `{0}`")]
    DuplicateComponentId(String),

    /// Una secuencia de teclas vacía no tiene sentido.
    #[error("secuencia de teclas vacía")]
    EmptyKeySequence,

    /// Se referenció un componente que no existe en la plantilla.
    #[error("componente desconocido: `{0}`")]
    UnknownComponent(String),

    /// Una acción `GoToPage` apunta a un índice que la plantilla no tiene.
    #[error("referencia de página inválida: {0}")]
    InvalidPageReference(usize),
}
