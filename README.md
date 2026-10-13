# Nolio

A self-hostable, offline-first workspace for docs and databases — in the spirit of Notion, owned by you.

**Nolio** is part of [FakeMake](https://fakemake.ir), a collection of open source tools you can run yourself. No subscriptions, no middlemen: your workspace lives in a single SQLite file on your machine or your own server.

## Principles

- **Offline or self-hosted.** The whole product is one Rust binary plus a SQLite database. Run it locally for offline use, or behind your own reverse proxy for your team.
- **Notion's interface model.** Pages contain typed blocks (text, headings, lists, to-dos, quotes, code, dividers). Everything is a block with a stable ID, a type, and an ordered position — the same foundation Notion's flexibility is built on.
- **Rust all the way down.** Axum for HTTP, rusqlite for storage, UUIDs for identity. Small surface, fast builds, no runtime dependencies.

## Quick start

```bash
cargo run
```

Then open http://localhost:8787. The database file (`nolio.db`) is created next to the binary — back it up, copy it, sync it with whatever you like.

### Configuration

| Env var    | Default    | Purpose            |
|------------|------------|--------------------|
| `NOLIO_PORT` | `8787`   | HTTP listen port   |
| `NOLIO_DB`  | `nolio.db` | SQLite database path |

## API

REST over JSON:

```
GET    /api/pages               list pages
POST   /api/pages               create page {parent_id?, title?, icon?}
GET    /api/pages/:id           get page
PATCH  /api/pages/:id           update page {title?, icon?}
DELETE /api/pages/:id           soft-delete page
GET    /api/blocks/:page_id     list blocks of a page (ordered)
POST   /api/blocks/:page_id     create block {type, content}
GET    /api/blocks/block/:id    get block
PATCH  /api/blocks/block/:id    update block {type?, content?}
DELETE /api/blocks/block/:id    soft-delete block
```

## Status

Early development. Currently shipping:

- [x] Pages with titles and icons
- [x] Typed blocks: paragraph, headings, lists, to-do, quote, code, divider
- [x] Slash-command type switching in the editor
- [x] Single SQLite file storage, soft deletes
- [ ] Nested blocks (toggle children, sub-page hierarchy in editor)
- [ ] Databases: properties, table/board views
- [ ] Realtime collaboration
- [ ] Full offline mode (local-first sync)
- [ ] Import/export

The long-term goal is full feature parity with Notion's core product, documented in the build blueprint.

## License

MIT — see [LICENSE](LICENSE).
