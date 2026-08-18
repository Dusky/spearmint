import './styles/fonts.css';
import './styles/tokens.css';
import './styles/hud.css';

import { createActions } from './state/actions';
import { createHud } from './ui/hud';
import { createWasmSurface } from './sim/wasmSurface';
import { ARENA_HEIGHT, ARENA_WIDTH, initialState } from './state/initialState';
import { bindKeyboard } from './ui/input';
import { Sim } from './sim/wasm';
import { startSimFeed } from './state/simFeed';
import { Store } from './state/store';
import type { GameState } from './state/types';

const store = new Store<GameState>(initialState());

const sim = await Sim.load(store.state.seed, ARENA_WIDTH, ARENA_HEIGHT);
const actions = createActions(store, sim);
const hud = createHud(createWasmSurface(sim), actions);

document.body.append(hud.root);
store.subscribe((state) => hud.update(state));
hud.update(store.state);

bindKeyboard(actions);
startSimFeed(store, sim);
