import { TOOLS } from '../state/types';
import type { Actions } from '../state/actions';

/**
 * Keyboard bindings.
 *
 * Tool keys 1–6 and `F` are from the handoff. The rest — `G` for the tile grid, `C`
 * and `B` for the two drawers, `Escape` to back out — are implementation choices:
 * the handoff says the grid is toggleable and that the panels become overlays, but
 * never says what opens them. Flagged in the README, not settled here.
 */
export function bindKeyboard(actions: Actions): () => void {
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.repeat || event.metaKey || event.ctrlKey) return;

    const index = Number.parseInt(event.key, 10) - 1;
    const tool = Number.isNaN(index) ? undefined : TOOLS[index];
    if (tool) {
      actions.selectTool(tool);
      return;
    }

    switch (event.key.toLowerCase()) {
      case 'f':
        actions.jumpToNotice();
        break;
      case 'g':
        actions.toggleTileGrid();
        break;
      case 'c':
        actions.toggleDrawer('capability');
        break;
      case 'b':
        actions.toggleDrawer('blueprints');
        break;
      case 'escape':
        // Back out of the drawer first, then the selection.
        if (document.querySelector('.drawer:not([hidden])')) actions.closeDrawer();
        else actions.clearSelection();
        break;
      default:
        return;
    }
  };

  window.addEventListener('keydown', onKeyDown);
  return () => window.removeEventListener('keydown', onKeyDown);
}
