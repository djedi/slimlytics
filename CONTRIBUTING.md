# Contributing

## Development loop

1. Create a focused branch.
2. Write one failing behavior test.
3. Run it and confirm the expected failure.
4. Implement the smallest passing change.
5. Refactor with the suite green.
6. Run `make test`, `make check`, and `make build`.
7. Open a pull request with behavior, privacy, migration, and deployment notes.

## Security and privacy review

Every collection-path change must answer:

- Can untrusted input reach SQL, logs, HTML, headers, or filesystem paths unsafely?
- Does this collect more identifying information than before?
- Are sensitive URL parameters still redacted?
- Is origin and site authorization enforced?
- Are duplicate and replayed events safe?
- Does denial or revocation of consent stop new browser events?

Never use production analytics payloads or credentials as test fixtures.

## Database changes

Migrations are append-only after release. Use explicit indexes and constraints, test against PostgreSQL, and document whether rollback requires restoring a backup.

## Commit format

Use conventional prefixes such as `feat:`, `fix:`, `test:`, `docs:`, `refactor:`, and `chore:`.

## Pull request checks

CI selects checks from the full PR diff. Backend or migration changes run Rust and database tests; frontend or tracker changes run web checks; CLI changes run CLI checks. Changes to `docs/openapi.json` run both backend and web checks because both compile it. Application and container changes also build and smoke-test the Compose stack. Prose-only documentation skips application builds. Unknown files, shared tooling, and CI changes select all gates; pushes to `main` always run the full suite.

The change selection rules are in `scripts/ci-changes.mjs`, with coverage run by `node --test scripts/ci-changes.test.mjs`. The job names remain `backend`, `web`, `cli`, and `container`; unaffected jobs appear as skipped. The existing restriction that fork PR code cannot execute on the persistent self-hosted runner remains in place.

Container checks use `compose.ci.yaml` to poll health every second instead of waiting for production polling intervals. The frontend Docker build skips duplicate checks only when the web job passed for the same revision; ordinary Docker builds still run their checks. Rust build dependencies and targets are cached in BuildKit mounts, with the executable copied into the image outside the cache. The first build fills the cache; later source changes reuse dependency compilation.
