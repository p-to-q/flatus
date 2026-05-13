<!-- one or two sentences. plain English. -->

## what

## why

## checklist

- [ ] `cargo fmt --all` clean
- [ ] `cargo clippy -p fart-synth -- -D warnings` clean
- [ ] `cargo test -p fart-synth` green
- [ ] If you touched synthesis: re-ran `cargo run --bin generate-goldens` and re-pinned `fixtures/golden/manifest.json`
- [ ] If you touched `src/safety.rs`: added a line to `PLAN.md` §14 (history)
- [ ] README still reads in OpenWhip voice; PLAN / ACOUSTICS / ENGINEERING still read in Wittgenstein voice
