# Provenance

Contents of `design_handoff_particle_factory_ui.zip`, vendored unchanged as the reference
for the Particle Factory in-game UI.

| File | What it is |
|---|---|
| `HANDOFF.md` | The handoff document (the zip's own `README.md`, renamed to avoid shadowing a repo README). Layout spec, tokens, interactions, state, and open questions. |
| `Particle Factory UI.dc.html` | The design prototype. Read the markup as the layout spec; the trailing script is a stand-in falling-sand sim written only so the chrome could be judged against live content. Do not port it. |
| `support.js` | Authoring-tool runtime, generated. Not part of the design; kept only so the prototype can be opened in its authoring environment. Ignore it. |

The prototype does not run standalone outside that authoring environment.

The handoff refers to "the project's own design & technical spec" (TypeScript/WebGL2 client,
Rust→WASM deterministic fixed-point sim, server-authoritative economy). That spec is not in
this repository.
