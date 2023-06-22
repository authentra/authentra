import { Api } from '$lib/api';
import { ApplicationApi, ApplicationGroupApi } from '$lib/api/developer';
import { OAuthApi } from '$lib/server/apis/oauth';
import { UserApi } from '$lib/server/apis/user';
import { INTERNAL_API_URL, checkAdmin, checkAuth, checkDeveloper } from '$lib/server/utils';
import { API_URL } from '$lib/utils';

if (!INTERNAL_API_URL || !API_URL) {
  throw Error()
}

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
  const api = new Api(INTERNAL_API_URL, event.fetch, event.cookies);
  const cookie = event.cookies.get('session_token');
  if (cookie) {
    const jwt_cookie = event.cookies.get('jwt');
    if (jwt_cookie) {
      api.tokenStore.set(jwt_cookie)
    } else {
      await api.refreshToken();
    }
    event.locals.user = await api.me();
  } else {
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
    checkAdmin(event.url, event.locals)
  }
  const response = await resolve(event, {
    transformPageChunk: ({ html }) => html.replace('%unocss-svelte-scoped.global%', 'unocss_svelte_scoped_global_styles'),
  })
  return response
}