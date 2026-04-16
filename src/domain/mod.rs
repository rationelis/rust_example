//! Business logic and domain entities.
//!
//! This is the "domain layer" in Clean Architecture. It contains:
//! - **Entities**: Core business objects (`Note`)
//! - **Services**: Business operations (`NoteService`)
//!
//! This layer has no knowledge of HTTP, databases, or any I/O. It depends only
//! on traits (ports) defined in the persistence layer, not concrete implementations.

pub mod note;
