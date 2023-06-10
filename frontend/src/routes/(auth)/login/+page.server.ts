import { fail, redirect } from "@sveltejs/kit";
import type { Actions, PageServerLoad } from "./$types";
import { extractRedirect, jsonBody } from "$lib/utils";
import * as set_cookie_parser from 'set-cookie-parser';

export const actions: Actions = {
    default: async ({ url, request, locals, cookies, fetch}) => {
        const form = await request.formData();
        const user = form.get('user') as string;
        const password = form.get('password') as string;
        //const res = await locals.api.auth.login(user, password);
        const res = await locals.api._extendResponse(await locals.api.svelteFetch(locals.api.makeLoc("/auth/browser/login"), {
            method: 'post',
            ...jsonBody({user, password})
        }))
        if (!res.api) {
            return fail(res.status, {success: false, message: await res.text()})
        }
        if (res.api.success) {
            //@ts-expect-error
            for (const str of set_cookie_parser.splitCookiesString(res.headers.get('set-cookie'))) {
                const { name, value, ...options } = set_cookie_parser.parseString(str);
                //@ts-expect-error
                cookies.set(name, value, { ...options });
              }
            throw redirect(303, extractRedirect(url.searchParams))
        } else {
            return fail(res.status, {success: false, message: res.api.message})
        }
    }
};

export const load: PageServerLoad = async ({url, locals}) => {
    if (locals.user) {
        throw redirect(303, extractRedirect(url.searchParams))
    }
};