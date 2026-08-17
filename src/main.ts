import './styles/fonts.css';
import './styles/tokens.css';
import './styles/hud.css';

import { createActions } from './state/actions';
import { createHud } from './ui/hud';
import { createPlaceholderSurface } from './sim/placeholderSurface';
import { initialState, startPlaceholderFeed } from './state/placeholder';
import { bindKeyboard } from './ui/input';
import { Store } from './state/store';
import type { GameState } from './state/types';

const store = new Store<GameState>(initialState());
const actions = createActions(store);

// Swap this for the WebGL2 renderer over the Rust/WASM sim; nothing else changes.
const surface = createPlaceholderSurface();

const hud = createHud(surface, actions);
document.body.append(hud.root);

store.subscribe((state) => hud.update(state));
hud.update(store.state);

bindKeyboard(actions);
startPlaceholderFeed(store);
