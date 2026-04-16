//! Repository traits (ports) for the persistence layer.
//!
//! In Clean Architecture, this module defines the "port" — an interface that the
//! domain layer uses without knowing the concrete implementation. The actual
//! implementations (adapters) like `InMemoryNoteRepository` live in submodules.
//!
//! This is **Dependency Inversion**: the domain depends on this trait (abstraction),
//! not on concrete storage implementations.

pub mod memory;

use error_stack::Report;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::note::Note;

/// Persistence layer errors.
///
/// These errors represent what can go wrong at the storage level.
/// Domain-relevant errors (like `NotFound`, `Forbidden`) are kept separate
/// from infrastructure errors (`StorageError`) which maps to `Unexpected` in the domain layer.
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// The requested note was not found.
    #[error("Note with id {note_id} not found")]
    NotFound { note_id: Uuid },

    /// The note does not belong to the requesting user.
    #[error("Note {note_id} does not belong to user")]
    Forbidden { note_id: Uuid },

    /// Infrastructure/storage error (maps to `Unexpected` in domain layer).
    #[error("Storage operation failed")]
    StorageError,
}

/// Contract for note storage. Implementations can be in-memory, PostgreSQL, etc.
///
/// The trait bounds serve specific purposes:
/// - `Clone`: Allows sharing the repository across handlers
/// - `Send + Sync`: Required for async runtimes (multiple threads)
/// - `'static`: Required for use in async contexts and Poem's data extraction
///
/// All methods return `Result<T, Report<RepositoryError>>` following the error-stack pattern.
pub trait NoteRepository: Clone + Send + Sync + 'static {
    fn list(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Note>, Report<RepositoryError>>> + Send;

    fn get(
        &self,
        user_id: &str,
        note_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Note, Report<RepositoryError>>> + Send;

    fn create(
        &self,
        note: Note,
    ) -> impl std::future::Future<Output = Result<Note, Report<RepositoryError>>> + Send;

    fn update(
        &self,
        user_id: &str,
        note: Note,
    ) -> impl std::future::Future<Output = Result<Note, Report<RepositoryError>>> + Send;

    fn delete(
        &self,
        user_id: &str,
        note_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), Report<RepositoryError>>> + Send;
}

#[cfg(test)]
pub mod mock {
    use super::*;

    mockall::mock! {
        pub NoteRepository {}

        impl Clone for NoteRepository {
            fn clone(&self) -> Self;
        }

        impl NoteRepository for NoteRepository {
            async fn list(&self, user_id: &str) -> Result<Vec<Note>, Report<RepositoryError>>;
            async fn get(&self, user_id: &str, note_id: Uuid) -> Result<Note, Report<RepositoryError>>;
            async fn create(&self, note: Note) -> Result<Note, Report<RepositoryError>>;
            async fn update(&self, user_id: &str, note: Note) -> Result<Note, Report<RepositoryError>>;
            async fn delete(&self, user_id: &str, note_id: Uuid) -> Result<(), Report<RepositoryError>>;
        }
    }
}
