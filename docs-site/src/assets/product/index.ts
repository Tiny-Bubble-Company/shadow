// Product screenshots used on the home page and in the docs (via ProductShot.astro).
//
// Real captures of `/shadow/ui?demo=1` (1280x800, and the 420x640 tray popover, both at 2x)
// and `shadow top --demo` (120x36). Astro optimises them to responsive WebP at build time.
import dashboardLight from './dashboard-light.png';
import dashboardDark from './dashboard-dark.png';
import tray from './tray.png';
import terminal from './terminal.png';

export const product = { dashboardLight, dashboardDark, tray, terminal };
