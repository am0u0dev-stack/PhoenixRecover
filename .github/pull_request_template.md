## What does this change?

<!-- One or two sentences. -->

## Why?

<!-- The problem this solves. Link the issue if there is one: Closes #123 -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] New file signature / format support
- [ ] Performance
- [ ] Documentation
- [ ] Build / CI

## Checklist

- [ ] The source device is still opened **read-only** — no write path added
- [ ] `cd src-tauri && cargo test` passes
- [ ] `cargo fmt` applied
- [ ] Built and smoke-tested with `npm run tauri dev` (if UI changed)
- [ ] Documentation updated (README / CHANGELOG)

## Screenshots

<!-- If the UI changed, add before/after screenshots. -->

## Notes for reviewers

<!-- Anything that needs extra attention: risky code paths, perf-sensitive
     loops, platform-specific behaviour. -->
