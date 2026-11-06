# nom
Recipe website

## Configuration

| Variable | Meaning |
|---|---|
| `NOM_USERNAME`, `NOM_PASSWORD` | HTTP Basic auth credentials. Required, unless `NOM_NO_AUTH` is set. |
| `NOM_NO_AUTH` | `1` or `true` (any case) runs the site without authentication. Any other value is an error, and so is combining it with `NOM_USERNAME` or `NOM_PASSWORD`. |
| `NOM_DB` | Path to the SQLite database. Defaults to `nom.db`. |

Photos added to recipes are stored in the database too (`images` table, at most 20 MB per
upload), so backing up `NOM_DB` backs up everything. Deleting shrinks the file (SQLite's
`auto_vacuum`; switching an older database over runs a `VACUUM` once, at startup).

An empty variable counts as unset. On a configuration error the server prints the problem and exits with code 1.

## Development

Needs [cargo-leptos](https://github.com/leptos-rs/cargo-leptos) (`cargo install --locked cargo-leptos`)
and the `wasm32-unknown-unknown` target. It downloads dart-sass itself.

```sh
cargo leptos watch                  # http://127.0.0.1:3000, rebuilds on changes
cargo test --lib --features ssr     # unit tests
```

## Docker

```sh
docker build -t nom .
```

The image listens on port 8080 and keeps the database at `/app/nom.db`, unless `NOM_DB` says
otherwise. Set the credentials (or `NOM_NO_AUTH`) as described above.
