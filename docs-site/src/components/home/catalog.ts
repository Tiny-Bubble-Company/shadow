// The size of Shadow's AI service catalog (crates/shadow-core/catalog.json, the file the proxy
// itself reads), rounded down to the nearest ten for copy such as "280+ AI services".
import catalog from '../../../../crates/shadow-core/catalog.json';

export const CATALOG_COUNT = (catalog as unknown[]).length;
export const CATALOG_LABEL = `${Math.floor(CATALOG_COUNT / 10) * 10}+ AI services`;
