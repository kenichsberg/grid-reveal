/**
 * Current-user state, hydrated once per page load from `GET /auth/me`.
 */
import { api, ApiError } from '$lib/api/client';
import type { CurrentUser } from '$lib/api/types';

class AuthStore {
  user = $state<CurrentUser | null>(null);
  /** True until the first hydration settles, so guards do not redirect early. */
  loading = $state(true);

  get isAuthenticated() {
    return this.user !== null;
  }

  get isAdmin() {
    return this.user?.is_admin ?? false;
  }

  async hydrate() {
    try {
      this.user = await api.me();
    } catch (error) {
      // A 401 here is the ordinary "not logged in" case, not a failure.
      if (!(error instanceof ApiError) || !error.isUnauthorized) {
        console.error('could not load the current user', error);
      }
      this.user = null;
    } finally {
      this.loading = false;
    }
  }

  async login(username: string, password: string) {
    this.user = await api.login({ username, password });
  }

  /** Adopts the session an invite established, without a second round trip. */
  adopt(user: CurrentUser) {
    this.user = user;
    this.loading = false;
  }

  async logout() {
    try {
      await api.logout();
    } finally {
      this.user = null;
    }
  }

  /** Called when any request returns 401 mid-session. */
  clear() {
    this.user = null;
  }
}

export const auth = new AuthStore();
