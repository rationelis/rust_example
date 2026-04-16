# Notes API

> **📚 Educational Project** — A minimal Rust web service demonstrating **Clean Architecture** (Hexagonal / Ports & Adapters). Data is stored **in-memory only** and will be lost when the server stops.

## Learning Objectives

- **Dependency Inversion**: Core business logic depends on traits, not concrete implementations
- **Layered Architecture**: Clear separation between HTTP, domain, and persistence concerns
- **Generic Services**: `NoteService<R: NoteRepository>` accepts any repository implementation
- **Error Handling**: Each layer defines its own error types with clean mapping between layers

## Architecture

```
src/
├── main.rs              # Entry point, server setup, CLI
├── auth.rs              # JWT authentication (with dev mode bypass)
├── adapter/web/         # HTTP handlers, DTOs, OpenAPI spec
├── domain/              # Business logic, entities (no I/O knowledge)
└── persistence/         # Repository trait + in-memory implementation
```

| Layer           | Responsibility                          |
| --------------- | --------------------------------------- |
| **adapter**     | HTTP ↔ Domain translation               |
| **domain**      | Business rules (framework-agnostic)     |
| **persistence** | Data access abstraction (port + adapter)|

## Running

```bash
cargo test                           # Run tests
cargo run -- server --dev-mode       # Start with dev auth (no JWT required)
cargo run -- spec                    # Print OpenAPI spec to stdout
```

## API

| Method | Path         | Description       |
| ------ | ------------ | ----------------- |
| GET    | `/notes`     | List user's notes |
| POST   | `/notes`     | Create note       |
| GET    | `/notes/:id` | Get note          |
| PUT    | `/notes/:id` | Update note       |
| DELETE | `/notes/:id` | Delete note       |

## Example (dev mode)

```bash
# Create a note
curl -X POST http://localhost:3000/notes \
  -H "Content-Type: application/json" \
  -H "X-Dev-User-Id: user-1" \
  -d '{"title": "Hello", "content": "World"}'

# List notes
curl http://localhost:3000/notes -H "X-Dev-User-Id: user-1"
```

> ⚠️ **Dev mode** bypasses JWT authentication and should never be used in production.

## Further Reading

- [Clean Architecture (Robert C. Martin)](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Hexagonal Architecture (Alistair Cockburn)](https://alistair.cockburn.us/hexagonal-architecture/)

## License

MIT