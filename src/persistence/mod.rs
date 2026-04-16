//! Repository traits (ports) for the persistence layer.
//!
//! In Clean Architecture, this module defines the "port" — an interface that the
//! domain layer uses without knowing the concrete implementation. The actual
//! implementations (adapters) like `InMemoryNoteRepository` live in submodules.
//!
//! This is **Dependency Inversion**: the domain depends on this trait (abstraction),
//! not on concrete storage implementations.

pub mod memory;

use thiserror::Error;
use uuid::Uuid;

use crate::domain::note::Note;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Note not found")]
    NotFound,
    #[error("Note does not belong to user")]
    Forbidden,
    #[error("Storage error: {0}")]
    Storage(String),
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Contract for note storage. Implementations can be in-memory, PostgreSQL, etc.
///
/// The trait bounds serve specific purposes:
/// - `Clone`: Allows sharing the repository across handlers
/// - `Send + Sync`: Required for async runtimes (multiple threads)
/// - `'static`: Required for use in async contexts and Poem's data extraction
pub trait NoteRepository: Clone + Send + Sync + 'static {
    fn list(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = RepositoryResult<Vec<Note>>> + Send;

    fn get(
        &self,
        user_id: &str,
        note_id: Uuid,
    ) -> impl std::future::Future<Output = RepositoryResult<Note>> + Send;

    fn create(
        &self,
        note: Note,
    ) -> impl std::future::Future<Output = RepositoryResult<Note>> + Send;

    fn update(
        &self,
        user_id: &str,
        note: Note,
    ) -> impl std::future::Future<Output = RepositoryResult<Note>> + Send;

    fn delete(
        &self,
        user_id: &str,
        note_id: Uuid,
    ) -> impl std::future::Future<Output = RepositoryResult<()>> + Send;
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
            async fn list(&self, user_id: &str) -> RepositoryResult<Vec<Note>>;
            async fn get(&self, user_id: &str, note_id: Uuid) -> RepositoryResult<Note>;
            async fn create(&self, note: Note) -> RepositoryResult<Note>;
            async fn update(&self, user_id: &str, note: Note) -> RepositoryResult<Note>;
            async fn delete(&self, user_id: &str, note_id: Uuid) -> RepositoryResult<()>;
        }
    }
}
