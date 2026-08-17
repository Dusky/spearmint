import { createBlueprintsDrawer } from './drawers/blueprints';
import { createCapabilityDrawer } from './drawers/capability';
import { createInspector } from './inspector';
import { createMarquee } from './marquee';
import { createNotice } from './notice';
import { createStatusBar } from './statusBar';
import { createToolColumn } from './toolColumn';
import { createTopBar } from './topBar';
import { createViewport } from './viewport';
import { el } from './dom';
import type { Actions } from '../state/actions';
import type { Component } from './component';
import type { GameState } from '../state/types';
import type { SimSurface } from '../sim/surface';

/**
 * The whole frame: top bar, body (tool column + viewport), status bar.
 *
 * Fluid and full-viewport. The prototype's fixed width, outer border and radius
 * existed only because it sat on a page; in-game the HUD is edge-to-edge.
 */
export function createHud(surface: SimSurface, actions: Actions): Component {
  const topBar = createTopBar();
  const toolColumn = createToolColumn(actions);
  const viewport = createViewport(surface, actions);
  const statusBar = createStatusBar();

  // Everything below is positioned over the canvas.
  const marquee = createMarquee(viewport.root);
  const inspector = createInspector();
  const notice = createNotice({ dismiss: actions.dismissNotice });
  const capability = createCapabilityDrawer();
  const blueprints = createBlueprintsDrawer();

  const overlays = [marquee, inspector, notice, capability, blueprints];
  viewport.root.append(...overlays.map((component) => component.root));

  const components: readonly Component[] = [topBar, toolColumn, viewport, statusBar, ...overlays];

  const root = el('div', { class: 'hud' }, [
    topBar.root,
    el('div', { class: 'body' }, [toolColumn.root, viewport.root]),
    statusBar.root,
  ]);

  return {
    root,
    update(state: GameState) {
      for (const component of components) component.update(state);
    },
  };
}
