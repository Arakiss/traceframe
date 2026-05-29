## Summary

<!-- What changed, and why? -->

## Trace Contract

- [ ] Raw trace files remain the source of truth.
- [ ] Any ledger, report, index, or export remains rebuildable from trace files.
- [ ] Any event schema or CLI output change is documented.

## Testing

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
sh scripts/check-release-readiness.sh
```

## Notes

<!-- Known risks, rejected alternatives, or follow-up work. -->
