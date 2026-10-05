# tnoodle-rs

A faithful Rust port of [tnoodle-lib](https://github.com/thewca/tnoodle-lib), the official
World Cube Association scramble library. Given the same `java.util.Random` seed it produces
the same scrambles, solutions and SVG drawings as the Java library, byte for byte.

```rust
use tnoodle::java::JavaRandom;
use tnoodle::PuzzleRegistry;

let three = PuzzleRegistry::Three.scrambler();
let scramble = three.generate_wca_scramble(&mut JavaRandom::new(42));
let svg = three.draw_scramble(Some(&scramble), None)?.to_string();
```

For real scrambles, use `generate_scramble()`, which seeds a `SHA1PRNG` from OS entropy like
TNoodle does.

## Layout

| Module | Java source |
| --- | --- |
| `scrambles` | `org.worldcubeassociation.tnoodle.scrambles` (framework, registry, cacher) |
| `puzzle` | `...scrambles.puzzle` (cubes, clock, megaminx, pyraminx, skewb, square-1, FTO) |
| `min2phase`, `threephase`, `sq12phase`, `fto3phase` | the solver packages |
| `svg` | `svglite` |
| `analysis` | `scrambles.analysis` (the statistical scramble checks) |
| `java` | emulation of the JDK behaviour the output depends on |

The `java` module is what makes the port exact. It covers `java.util.Random`, `SHA1PRNG`,
`String.hashCode`/`compareTo`, `HashMap` iteration order (including resizes, copy-constructor
sizing and treeified bins), `PriorityQueue`, `Double.toString`, `Math.round` and HotSpot's
`sin`/`cos`/`acos`.

## Testing

```bash
cargo test -p tnoodle-rs
```

* **Differential tests** compare against fixtures recorded from the Java library
  (`tests/fixtures/*.json`): JDK primitives, every solver, and all 17 registry puzzles
  (scrambles, solutions, merging, SVG output, errors).
* **Ported JUnit tests** from tnoodle-lib (`tests/java_unit_tests.rs`), including the
  multi-threaded and cacher tests.
* **Property tests** (`tests/properties.rs`) check invariants for random seeds.
* Two slow statistical tests from `scrambles.analysis` are `#[ignore]`d. Run them with
  `cargo test -p tnoodle-rs --release -- --ignored`.

### Fixtures

The fixtures were recorded once from tnoodle-lib (run on JDK 25 without `-ea`, since TNoodle
runs in production with assertions disabled) and are kept as the reference. They are
excluded from the published crate, so these tests only run from this repository.

## Deviations from the Java library

Each of these is either unobservable through the public behaviour, or the Java behaviour
is itself nondeterministic:

* **Trigonometry**: `sin` and `cos` are correctly rounded. HotSpot's intrinsics agree in all
  but a handful of the recorded cases (one ulp, never visible in the SVG output).
* **Megaminx drawing order**: Java draws faces in the order of an identity-hashed `HashMap`,
  which changes between runs. This port draws them in face order.
* **`HashMap` tie-breaks**: Java orders distinct, non-comparable keys with equal hash codes
  in treeified bins by `System.identityHashCode`, which is nondeterministic. This port picks
  one of Java's possible orders.
* **Solver randomness**: the Pyraminx and Skewb solvers use an unseeded `new Random()` in
  Java. Here they use OS entropy by default, and `with_search_seed` makes them reproducible.
  The 3x3x3's 200 ms minimum search time can be removed for tests with
  `with_min_search_time(Duration::ZERO)`.
* **Omitted**: threephase table serialization (`Tools.initFrom`/`saveTo`), debug printing,
  and the GWT exporter annotations.
* **Errors**: invalid hex colors return `None` instead of throwing, and Java exceptions map
  to `Result` errors (`InvalidMoveError`, `InvalidScrambleError`) with the same messages.

## License

tnoodle-lib is licensed under the GPL-3.0, so this port, as a derivative work, is licensed
GPL-3.0-or-later.
