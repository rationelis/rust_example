//! In-memory repository implementation.

use std::collections::HashMap;
use std::sync::RwLock;

use uuid::Uuid;

use super::{NoteRepository, RepositoryError, RepositoryResult};
use crate::domain::note::Note;

#[derive(Debug)]
pub struct InMemoryNoteRepository {
    notes: RwLock<HashMap<Uuid, Note>>,
}

impl InMemoryNoteRepository {
    #[must_use]
    pub fn new() -> Self {
        Self {
            notes: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryNoteRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for InMemoryNoteRepository {
    fn clone(&self) -> Self {
        let notes = self.notes.read().expect("RwLock poisoned").clone();
        Self {
            notes: RwLock::new(notes),
        }
    }
}

impl NoteRepository for InMemoryNoteRepository {
    async fn list(&self, user_id: &str) -> RepositoryResult<Vec<Note>> {
        let notes = self
            .notes
            .read()
            .map_err(|e| RepositoryError::Storage(format!("Failed to acquire read lock: {e}")))?;

        let user_notes: Vec<Note> = notes
            .values()
            .filter(|note| note.user_id() == user_id)
            .cloned()
            .collect();

        Ok(user_notes)
    }

    async fn get(&self, user_id: &str, note_id: Uuid) -> RepositoryResult<Note> {
        let notes = self
            .notes
            .read()
            .map_err(|e| RepositoryError::Storage(format!("Failed to acquire read lock: {e}")))?;

        let note = notes.get(&note_id).ok_or(RepositoryError::NotFound)?;

        if note.user_id() != user_id {
            return Err(RepositoryError::Forbidden);
        }

        Ok(note.clone())
    }

    async fn create(&self, note: Note) -> RepositoryResult<Note> {
        let mut notes = self
            .notes
            .write()
            .map_err(|e| RepositoryError::Storage(format!("Failed to acquire write lock: {e}")))?;

        notes.insert(note.id(), note.clone());

        Ok(note)
    }

    async fn update(&self, user_id: &str, note: Note) -> RepositoryResult<Note> {
        let mut notes = self
            .notes
            .write()
            .map_err(|e| RepositoryError::Storage(format!("Failed to acquire write lock: {e}")))?;

        let existing = notes.get(&note.id()).ok_or(RepositoryError::NotFound)?;

        if existing.user_id() != user_id {
            return Err(RepositoryError::Forbidden);
        }

        notes.insert(note.id(), note.clone());

        Ok(note)
    }

    async fn delete(&self, user_id: &str, note_id: Uuid) -> RepositoryResult<()> {
        let mut notes = self
            .notes
            .write()
            .map_err(|e| RepositoryError::Storage(format!("Failed to acquire write lock: {e}")))?;

        let existing = notes.get(&note_id).ok_or(RepositoryError::NotFound)?;

        if existing.user_id() != user_id {
            return Err(RepositoryError::Forbidden);
        }

        notes.remove(&note_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_note(user_id: &str, title: &str) -> Note {
        Note::new(
            user_id.to_string(),
            title.to_string(),
            "Content".to_string(),
        )
    }

    #[tokio::test]
    async fn it_should_return_empty_list_for_new_repository() {
        let repo = InMemoryNoteRepository::new();
        let notes = repo.list("user-1").await.unwrap();
        assert!(notes.is_empty());
    }

    #[tokio::test]
    async fn it_should_only_list_notes_for_specified_user() {
        let repo = InMemoryNoteRepository::new();
        let note1 = create_test_note("user-1", "Note 1");
        let note2 = create_test_note("user-2", "Note 2");
        let note3 = create_test_note("user-1", "Note 3");

        repo.create(note1.clone()).await.unwrap();
        repo.create(note2).await.unwrap();
        repo.create(note3.clone()).await.unwrap();

        let user1_notes = repo.list("user-1").await.unwrap();

        assert_eq!(user1_notes.len(), 2);
        assert!(user1_notes.iter().any(|n| n.id() == note1.id()));
        assert!(user1_notes.iter().any(|n| n.id() == note3.id()));
    }

    #[tokio::test]
    async fn it_should_create_and_retrieve_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();

        let created = repo.create(note).await.unwrap();
        assert_eq!(created.id(), note_id);

        let retrieved = repo.get("user-1", note_id).await.unwrap();
        assert_eq!(retrieved.title(), "My Note");
    }

    #[tokio::test]
    async fn it_should_return_not_found_for_missing_note() {
        let repo = InMemoryNoteRepository::new();
        let result = repo.get("user-1", Uuid::new_v4()).await;
        assert!(matches!(result, Err(RepositoryError::NotFound)));
    }

    #[tokio::test]
    async fn it_should_return_forbidden_when_accessing_other_users_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        let result = repo.get("user-2", note_id).await;
        assert!(matches!(result, Err(RepositoryError::Forbidden)));
    }

    #[tokio::test]
    async fn it_should_update_existing_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "Original");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        let mut retrieved = repo.get("user-1", note_id).await.unwrap();
        retrieved.set_title("Updated".to_string());
        repo.update("user-1", retrieved).await.unwrap();

        let updated = repo.get("user-1", note_id).await.unwrap();
        assert_eq!(updated.title(), "Updated");
    }

    #[tokio::test]
    async fn it_should_return_forbidden_when_updating_other_users_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        let retrieved = repo.get("user-1", note_id).await.unwrap();
        let result = repo.update("user-2", retrieved).await;
        assert!(matches!(result, Err(RepositoryError::Forbidden)));
    }

    #[tokio::test]
    async fn it_should_delete_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        repo.delete("user-1", note_id).await.unwrap();

        let result = repo.get("user-1", note_id).await;
        assert!(matches!(result, Err(RepositoryError::NotFound)));
    }

    #[tokio::test]
    async fn it_should_return_forbidden_when_deleting_other_users_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        let result = repo.delete("user-2", note_id).await;
        assert!(matches!(result, Err(RepositoryError::Forbidden)));
    }

    #[tokio::test]
    async fn it_should_return_not_found_when_deleting_missing_note() {
        let repo = InMemoryNoteRepository::new();
        let result = repo.delete("user-1", Uuid::new_v4()).await;
        assert!(matches!(result, Err(RepositoryError::NotFound)));
    }
}
