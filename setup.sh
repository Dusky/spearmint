#!/usr/bin/env bash
#
# Sets up Particle Factory and starts the dev server.
#
#   ./setup.sh              install what is needed, then run the game
#   ./setup.sh --setup-only install what is needed and stop
#   ./setup.sh --check      also run the full test suite before starting
#
# It checks for Node and Rust rather than installing them — silently putting a
# toolchain on someone's machine is not a setup script's job. The one thing it does
# install is the wasm target, which is a component of a rustup you already have.
#
# Written for Arch, with fallbacks for other Linux and macOS. On Windows use WSL;
# nothing in the project itself is POSIX-specific.

set -euo pipefail

cd "$(dirname "$0")"

BOLD=$'\033[1m'; DIM=$'\033[2m'; RED=$'\033[31m'; GREEN=$'\033[32m'; RESET=$'\033[0m'
# No colour when piped to a file or a dumb terminal.
if [ ! -t 1 ]; then BOLD=""; DIM=""; RED=""; GREEN=""; RESET=""; fi

step() { printf '%s==>%s %s\n' "$BOLD" "$RESET" "$1"; }
note() { printf '    %s%s%s\n' "$DIM" "$1" "$RESET"; }
ok()   { printf '    %s✓%s %s\n' "$GREEN" "$RESET" "$1"; }
die()  { printf '%serror:%s %s\n' "$RED" "$RESET" "$1" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

SETUP_ONLY=0
RUN_CHECKS=0
for argument in "$@"; do
  case "$argument" in
    --setup-only) SETUP_ONLY=1 ;;
    --check) RUN_CHECKS=1 ;;
    -h|--help) sed -n '3,15p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "unknown option '$argument' (try --help)" ;;
  esac
done

# ── Node ──────────────────────────────────────────────────────────────────────

step "Checking Node.js"
if ! have node; then
  if have pacman; then
    hint="sudo pacman -S nodejs npm"
  elif [ "$(uname -s)" = "Darwin" ]; then
    hint="brew install node"
  else
    hint="see https://nodejs.org, or use nvm: https://github.com/nvm-sh/nvm"
  fi
  die "Node.js is not installed. $hint"
fi

node_major="$(node -p 'process.versions.node.split(".")[0]')"
# Vite 5 wants ^18 or >=20. Node 19 is both odd-numbered and unsupported by it.
if [ "$node_major" -lt 18 ] || [ "$node_major" -eq 19 ]; then
  die "Node $(node -v) is too old for Vite 5. Install Node 20 or newer."
fi
ok "node $(node -v)"

have npm || die "npm is missing, which is unusual alongside Node — reinstall Node."
ok "npm $(npm -v)"

# ── Rust ──────────────────────────────────────────────────────────────────────

step "Checking Rust"
if ! have cargo; then
  if have pacman; then
    die "Rust is not installed.
       sudo pacman -S rustup && rustup default stable

       The 'rustup' package is the one to want here: it honours the toolchain pin in
       rust-toolchain.toml and can add the wasm target. The 'rust' package also works,
       but see the note below about wasm32."
  fi
  die "Rust is not installed. Install rustup: https://rustup.rs
       (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)"
fi
ok "cargo $(cargo --version | cut -d' ' -f2)"

USING_RUSTUP=0
if have rustup; then
  USING_RUSTUP=1
  # rust-toolchain.toml pins the version, so cargo may download it on first use.
  note "the pinned toolchain downloads on first use, which can take a minute"
else
  note "rustup not found — using the system cargo"
  note "rust-toolchain.toml's version pin is ignored without rustup, which is fine"
  note "for running the game and only matters for byte-identical reproducibility"
fi

step "Checking the wasm target"
if [ "$USING_RUSTUP" -eq 1 ]; then
  if rustup target list --installed 2>/dev/null | grep -qx 'wasm32-unknown-unknown'; then
    ok "wasm32-unknown-unknown already installed"
  else
    note "installing wasm32-unknown-unknown"
    rustup target add wasm32-unknown-unknown
    ok "wasm32-unknown-unknown installed"
  fi
else
  # Arch's `rust` package ships cargo without rustup, and keeps the wasm standard
  # library in a separate package. Nothing to install here; the build below is the
  # real test, and it prints the fix if the target is missing.
  note "cannot check the target without rustup — the build will tell us"
fi

# ── Build ─────────────────────────────────────────────────────────────────────

step "Installing npm packages"
npm install --no-audit --no-fund
ok "packages installed"

step "Building the simulation for the browser"
# The dev server does this too, but doing it here means a Rust problem shows up now
# rather than as a blank page later.
if ! npm run sim:wasm; then
  printf '\n'
  if have pacman && [ "$USING_RUSTUP" -eq 0 ]; then
    die "the wasm build failed, most likely because the wasm32 standard library is
       missing. On Arch, with the 'rust' package rather than rustup:

           sudo pacman -S rust-wasm

       Or switch to rustup, which manages targets itself:

           sudo pacman -Rs rust && sudo pacman -S rustup && rustup default stable"
  fi
  die "the wasm build failed — see the cargo output above."
fi
ok "public/sim.wasm built ($(du -h public/sim.wasm | cut -f1))"

step "Typechecking the client"
npm run typecheck
ok "no type errors"

if [ "$RUN_CHECKS" -eq 1 ]; then
  step "Running the test suite"
  cargo test --workspace
  cargo clippy --workspace --all-targets -- -D warnings
  ok "tests and lints pass"
fi

# ── Go ────────────────────────────────────────────────────────────────────────

if [ "$SETUP_ONLY" -eq 1 ]; then
  step "Ready"
  note "start it with:  npm run dev"
  exit 0
fi

step "Starting the dev server"
cat <<'INSTRUCTIONS'

    Once it is up, open the URL below and try this:

      1. Draw two sloping walls into a basin        drag with the mouse (Draw, key 1)
      2. Switch to the spawner tool                 key 4
      3. Pick sand, click above the basin           first swatch, then click
      4. Pick water, click above the other side     third swatch, then click
      5. Watch the inspector on the right

    Yield rises when sand and water actually touch. The panel tells you when they
    do not — that is the whole loop, and the thing worth judging.

    Other keys:  2 erase · space+drag pan · G grid · alt+click pick material
    Stop with Ctrl-C.

INSTRUCTIONS

npm run dev
