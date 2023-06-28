import { building } from '$app/environment';
import { Api } from '$lib/api';
import { ApplicationApi, ApplicationGroupApi } from '$lib/api/developer';
import { OAuthApi } from '$lib/server/apis/oauth';
import { UserApi } from '$lib/server/apis/user';
import { INTERNAL_API_URL, checkAdmin, checkAuth, checkDeveloper } from '$lib/server/utils';
import { API_URL } from '$lib/utils';
import { error } from '@sveltejs/kit';
import { get, writable, type Writable } from 'svelte/store';

if ((!INTERNAL_API_URL || !API_URL) && !building) {
  throw Error()
}

export interface ApiStatus {
  online: boolean | undefined,
  last_check: number
}

const api_status_store = writable<ApiStatus>({
  online: undefined,
  last_check: Date.now()
});

export async function handleFetch({ event, request, fetch }) {
  const user_agent = event.request.headers.get('user-agent');
  if (user_agent) {
    request.headers.set('user-agent', user_agent);
  }
  if (request.url.startsWith(INTERNAL_API_URL)) {
    const cookie = event.request.headers.get('cookie');
    if (cookie) {
      request.headers.set('cookie', cookie);
    }
  }
  return await fetch(request);
}

export async function handle({ event, resolve }) {
  await check_api(api_status_store);
  const status = get(api_status_store);
  if (!status.online) {
    throw error(502, {message: "Can't connect to backend"})
  }
  const api = new Api(INTERNAL_API_URL, event.fetch, event.cookies);
  const cookie = event.cookies.get('session_token');
  if (cookie) {
    const jwt_cookie = event.cookies.get('jwt');
    if (jwt_cookie) {
      console.log('Setting store')
      api.tokenStore.set(jwt_cookie)
    } else {
      await api.refreshToken();
    }
    event.locals.user = await api.me();
  } else {
    console.log("No cookie")
    event.locals.user = null
  }
  event.locals.api = api;
  //@ts-expect-error
  event.locals.apis = { applications: new ApplicationApi(api), application_groups: new ApplicationGroupApi(api), users: new UserApi(api) };

  if (event.url.pathname.startsWith('/dash')) {
    checkAuth(event.url, event.locals)
  }

  if (event.url.pathname.startsWith('/admin')) {
    checkAdmin(event.url, event.locals)
  }
  if (event.url.pathname.startsWith('/developer')) {
    checkDeveloper(event.url, event.locals)
  }
  if (event.url.pathname.startsWith('/oauth/authorize')) {
    event.locals.apis.oauth = new OAuthApi(api)
    checkAuth(event.url, event.locals)
  }
  const response = await resolve(event, {
    transformPageChunk: ({ html }) => html.replace('%unocss-svelte-scoped.global%', 'unocss_svelte_scoped_global_styles'),
  })
  return response
}

async function check_api(store: Writable<ApiStatus>) {
  const now = Date.now()
  const status = get(store);
  if (now - status.last_check > 5000 || status.online === undefined) {
    const res = await check_api_internal();
    store.set({
      online: res.online,
      last_check: now + res.took
    })
  }
}
async function check_api_internal(): Promise<{
  online: boolean,
  took: number,
}> {
  const date = Date.now()
  console.log("Checking backend")
  try {
    const res = await fetch(INTERNAL_API_URL + '/internal/health');
    return { online: res.status == 200, took: Date.now()-date }
  } catch (e) {
    return { online: false, took: Date.now()-date }
  }
}