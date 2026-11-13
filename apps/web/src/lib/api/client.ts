/**
 * API client.
 *
 * Every request carries the session cookie, which is why `credentials` is
 * always `include`: the API lives on a sibling subdomain, so the browser will
 * not attach the cookie otherwise.
 */
import { PUBLIC_API_BASE_URL } from '$env/static/public';

import type {
  AddMemberRequest,
  AdminScreenEntry,
  ChangePasswordRequest,
  CloneScreenRequest,
  CreateEntryRequest,
  CreateGoalRequest,
  CreateScreenRequest,
  CreateUserRequest,
  CurrentUser,
  DeleteScreenRequest,
  EntrySummary,
  GeometryPreview,
  GoalSummary,
  LoginRequest,
  MemberSummary,
  PictureSummary,
  RemovalImpact,
  RemoveMemberRequest,
  ScreenDetail,
  ScreenList,
  SetPasswordRequest,
  SubmitEntryResponse,
  UpdateEntryRequest,
  UpdateScreenRequest,
  UserSummary
} from './types';

const BASE = (PUBLIC_API_BASE_URL ?? 'http://localhost:8080').replace(/\/$/, '');

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
  }

  get isUnauthorized() {
    return this.status === 401;
  }

  /** A screen that is not yours is indistinguishable from one that does not exist. */
  get isNotFound() {
    return this.status === 404;
  }
}

/** Called when any request comes back 401, so the shell can send the user to login. */
let onUnauthorized: (() => void) | null = null;
export function setUnauthorizedHandler(handler: () => void) {
  onUnauthorized = handler;
}

type RequestOptions = {
  method?: string;
  body?: unknown;
  /** Suppresses the global 401 handler, for probes that expect to fail. */
  quiet?: boolean;
};

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, quiet = false } = options;

  const init: RequestInit = {
    method,
    credentials: 'include',
    headers: body === undefined ? {} : { 'Content-Type': 'application/json' }
  };
  if (body !== undefined) init.body = JSON.stringify(body);

  let response: Response;
  try {
    response = await fetch(`${BASE}${path}`, init);
  } catch {
    throw new ApiError(0, 'network', 'Could not reach the server.');
  }

  if (response.status === 401 && !quiet) onUnauthorized?.();

  if (!response.ok) {
    let code = 'error';
    let message = `Request failed (${response.status}).`;
    try {
      const parsed = await response.json();
      if (typeof parsed?.error === 'string') code = parsed.error;
      if (typeof parsed?.message === 'string') message = parsed.message;
      // The hard-delete refusal reports how many screens still use a picture.
      if (typeof parsed?.screen_count === 'number') {
        code = 'picture_in_use';
        message = `${parsed.screen_count} screen${parsed.screen_count === 1 ? '' : 's'} still use this picture.`;
      }
    } catch {
      /* non-JSON error body */
    }
    throw new ApiError(response.status, code, message);
  }

  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

/** Multipart upload; the browser sets the boundary, so no Content-Type here. */
async function upload<T>(path: string, file: File): Promise<T> {
  const form = new FormData();
  form.append('file', file);

  let response: Response;
  try {
    response = await fetch(`${BASE}${path}`, { method: 'POST', credentials: 'include', body: form });
  } catch {
    throw new ApiError(0, 'network', 'Could not reach the server.');
  }

  if (response.status === 401) onUnauthorized?.();
  if (!response.ok) {
    let message = `Upload failed (${response.status}).`;
    let code = 'error';
    try {
      const parsed = await response.json();
      if (typeof parsed?.message === 'string') message = parsed.message;
      if (typeof parsed?.error === 'string') code = parsed.error;
    } catch {
      /* non-JSON error body */
    }
    throw new ApiError(response.status, code, message);
  }
  return (await response.json()) as T;
}

/** Joins an API-relative picture path with the API base; absolute URLs pass through. */
export function pictureUrl(url: string): string {
  return /^https?:\/\//.test(url) ? url : `${BASE}${url}`;
}

export const api = {
  // Auth
  login: (body: LoginRequest) => request<CurrentUser>('/auth/login', { method: 'POST', body }),
  logout: () => request<unknown>('/auth/logout', { method: 'POST' }),
  me: () => request<CurrentUser>('/auth/me', { quiet: true }),
  changePassword: (body: ChangePasswordRequest) =>
    request<unknown>('/auth/password', { method: 'POST', body }),

  // Users (admin)
  listUsers: () => request<UserSummary[]>('/users'),
  createUser: (body: CreateUserRequest) => request<UserSummary>('/users', { method: 'POST', body }),
  setPassword: (id: string, body: SetPasswordRequest) =>
    request<unknown>(`/users/${id}/password`, { method: 'POST', body }),

  // Pictures (admin)
  listPictures: () => request<PictureSummary[]>('/pictures'),
  uploadPicture: (file: File) => upload<PictureSummary>('/pictures', file),
  softDeletePicture: (id: string) => request<unknown>(`/pictures/${id}`, { method: 'DELETE' }),
  hardDeletePicture: (id: string) =>
    request<unknown>(`/pictures/${id}?hard=true`, { method: 'DELETE' }),

  // Screens
  listScreens: () => request<ScreenList>('/screens'),
  /** Admin only: every screen, including ones the caller does not belong to. */
  listAllScreens: () => request<AdminScreenEntry[]>('/admin/screens'),
  getScreen: (slug: string) => request<ScreenDetail>(`/screens/${slug}`),
  createScreen: (body: CreateScreenRequest) =>
    request<ScreenDetail>('/screens', { method: 'POST', body }),
  updateScreen: (slug: string, body: UpdateScreenRequest) =>
    request<ScreenDetail>(`/screens/${slug}`, { method: 'PATCH', body }),
  deleteScreen: (slug: string, body: DeleteScreenRequest) =>
    request<unknown>(`/screens/${slug}`, { method: 'DELETE', body }),
  cloneScreen: (slug: string, body: CloneScreenRequest) =>
    request<ScreenDetail>(`/screens/${slug}/clone`, { method: 'POST', body }),
  archiveScreen: (slug: string) => request<unknown>(`/screens/${slug}/archive`, { method: 'POST' }),
  unarchiveScreen: (slug: string) =>
    request<unknown>(`/screens/${slug}/unarchive`, { method: 'POST' }),
  geometryPreview: (params: {
    picture_id: string;
    target_total_minutes: number;
    unit_minutes: number;
    cols?: number;
  }) => {
    const query = new URLSearchParams({
      picture_id: params.picture_id,
      target_total_minutes: String(params.target_total_minutes),
      unit_minutes: String(params.unit_minutes)
    });
    if (params.cols !== undefined) query.set('cols', String(params.cols));
    return request<GeometryPreview>(`/screens/geometry-preview?${query}`);
  },

  // Membership (admin)
  listMembers: (slug: string) => request<MemberSummary[]>(`/screens/${slug}/members`),
  addMember: (slug: string, body: AddMemberRequest) =>
    request<unknown>(`/screens/${slug}/members`, { method: 'POST', body }),
  removalImpact: (slug: string, userId: string) =>
    request<RemovalImpact>(`/screens/${slug}/members/${userId}/removal-impact`),
  removeMember: (slug: string, userId: string, body: RemoveMemberRequest) =>
    request<unknown>(`/screens/${slug}/members/${userId}`, { method: 'DELETE', body }),

  // Entries
  listEntries: (slug: string) => request<EntrySummary[]>(`/screens/${slug}/entries`),
  createEntry: (slug: string, body: CreateEntryRequest) =>
    request<SubmitEntryResponse>(`/screens/${slug}/entries`, { method: 'POST', body }),
  updateEntry: (id: string, body: UpdateEntryRequest) =>
    request<EntrySummary>(`/entries/${id}`, { method: 'PATCH', body }),
  deleteEntry: (id: string) => request<unknown>(`/entries/${id}`, { method: 'DELETE' }),

  // Goals
  listGoals: (slug: string) => request<GoalSummary[]>(`/screens/${slug}/goals`),
  createGoal: (slug: string, body: CreateGoalRequest) =>
    request<GoalSummary>(`/screens/${slug}/goals`, { method: 'POST', body }),
  deleteGoal: (id: string) => request<unknown>(`/goals/${id}`, { method: 'DELETE' })
};
