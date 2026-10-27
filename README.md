# sml

Finite state machines for Rust with a transition-table DSL modelled on
[Boost.SML](https://github.com/boost-ext/sml). The table compiles to plain enum
dispatch.

```rust
use sml::{sml, Sm};

#[derive(Default)]
struct Ctx { opened: u32, locked: bool }

struct Open;
struct Close;
struct Kick;

fn is_unlocked(ctx: &Ctx) -> bool { !ctx.locked }
fn count(ctx: &mut Ctx, _: &Open) { ctx.opened += 1; }

sml! {
    machine Door<Ctx>;
    *Closed + Open [is_unlocked] / count = Opened,
     Opened + Close                      = Closed,
     Opened + Kick                       = X,
}

let mut sm = Sm::<Door>::new(Ctx::default());
assert!(sm.process_event(Open));
assert!(sm.is(DoorState::Opened));
```

## From Boost.SML

| Boost.SML                                  | sml                                     |
|--------------------------------------------|-----------------------------------------|
| `*"idle"_s + event<e1> [g] / a = "s1"_s`   | `*Idle + E1 [g] / a = S1`               |
| `[g1 && !g2]`                              | `[g1 && !g2]`                           |
| `/ (a1, a2)`                               | `/ (a1, a2)`                            |
| `"s"_s + on_entry<_> / a`                  | `S + on_entry / a`                      |
| `"s"_s + on_exit<_> / a`                   | `S + on_exit / a`                       |
| `"s"_s [g] = "t"_s` (anonymous)            | `S [g] = T`                             |
| `"s"_s + event<e> / a` (internal)          | `S + E / a`                             |
| `= X`                                      | `= X`                                   |
| `+ unexpected_event<_>` / `<e>`            | `+ unexpected_event<_>` / `<E>`         |
| `state<sub>` (composite)                   | `state<Sub>`                            |
| `*"idle"_s(H)` (history)                   | `*Idle(H)`                              |
| `sm<T> sm{deps...}`                        | `Sm::<T>::new(ctx)`                     |
| `sm.process_event(e{})`                    | `sm.process_event(E)`                   |
| `sm.is("s1"_s)`                            | `sm.is(TState::S1)`                     |
| `sm.is<decltype(state<sub>)>("a"_s)`       | `sm.sub::<Sub>().state() == SubState::A`|

Not supported: orthogonal regions (multiple `*`), `defer`/`process` queues,
type-based dependency injection.

### Differences

- **Dependencies**: guards and actions receive one context struct instead of
  injected dependencies. Guards are `Fn(&Ctx, &Event) -> bool` or `Fn(&Ctx) -> bool`;
  actions are `Fn(&mut Ctx, &Event)` or `Fn(&mut Ctx)`.
- **Events without a payload type**: entry, exit and anonymous rows get no event;
  `unexpected_event<_>` gets `&dyn Any`.
- **Closures** need explicit parameter types and, as actions, parentheses:
  `/ (|ctx: &mut Ctx| ctx.n += 1)`.
- **Generated names**: `machine Door` defines `struct Door` and `enum DoorState`.

Full syntax and semantics: crate docs (`cargo doc --open -p sml`).

## Diagrams

Enable the `diagram` feature and every `sml!` machine gets a state diagram, laid out
and rendered in pure Rust by the zero-dependency `sml-diagram` crate (no Graphviz,
no build-time files):

- **rustdoc**: the SVG is embedded in the machine's documentation and follows the
  docs theme's colours. Composite states are shown by reference (`state<Inner>`).
- **runtime**: `Door::graph()` returns the graph with sub-machines nested inside
  their composite states; `Door::diagram_svg()` renders a standalone SVG with its own
  light/dark palette.

```rust
use sml::Diagram;
std::fs::write("door.svg", Door::diagram_svg())?;
```

Drawing follows UML: a filled dot (or `H` for history) marks the start, `X` is a
bullseye, and entry/exit actions and internal transitions are listed inside the state.

`sml-diagram` is independent of `sml`: build a `Graph` of nodes, edges and nested
graphs and call `render_svg`.

## Logging

Enable the `tracing` feature to emit [`tracing`](https://docs.rs/tracing) events
(target `sml`): processed events and state changes at `DEBUG`, guards and actions at
`TRACE`. Without it the hooks compile to nothing.

## Development

Crates: `sml` (runtime), `sml-macros` (the `sml!` proc macro), `sml-diagram`
(layout + SVG).

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets -- -D warnings
cargo test --all-features
cargo deny check
scripts/mutants.sh
```

`scripts/mutants.sh` runs mutation testing per crate (each against the tests that
cover it) and in both feature modes; see the script for why.

Diagram output is snapshot-tested: reference SVGs live in
`crates/sml-diagram/tests/snapshots/`. After an intentional layout or drawing change,
run `UPDATE_SNAPSHOTS=1 cargo test -p sml-diagram --test snapshots` and look at the
regenerated files before committing them.

Diagnostics for malformed tables are snapshot-tested with `trybuild`
(`crates/sml/tests/ui`); refresh with `TRYBUILD=overwrite cargo test --test ui`.
