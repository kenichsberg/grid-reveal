/**
 * What this installation calls itself, hydrated once per page load from
 * `GET /settings`.
 *
 * Separate from the auth store because the name is needed before there is a
 * session: the login and invite pages print it to people who have not signed
 * in yet.
 */
import { api } from '$lib/api/client';

/**
 * What the application is called when the API has not answered yet, or cannot.
 * A name must always render, so there is no empty state to design around.
 */
const FALLBACK_NAME = 'Grid Reveal';

class SiteStore {
  name = $state(FALLBACK_NAME);
  /** True until the first hydration settles, so no page shows the fallback. */
  loading = $state(true);

  async hydrate() {
    try {
      this.name = (await api.settings()).app_name;
    } catch (error) {
      // The name is decoration, not access: an unreachable API should leave
      // the page usable under the built-in name rather than blank.
      console.error('could not load the site settings', error);
    } finally {
      this.loading = false;
    }
  }

  /** Administrator-only; the header retitles itself without a reload. */
  async rename(appName: string) {
    this.name = (await api.updateSettings({ app_name: appName })).app_name;
  }
}

export const site = new SiteStore();

/**
 * `pageTitle('Screens')` -> `Screens · Daimoku`; `pageTitle()` -> `Daimoku`.
 *
 * A function rather than a template in each page, so that renaming the
 * installation renames every browser tab with it.
 */
export function pageTitle(part?: string): string {
  return part ? `${part} · ${site.name}` : site.name;
}
