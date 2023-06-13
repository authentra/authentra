import { fail, redirect } from "@sveltejs/kit";
import type { Actions, PageServerLoad } from "./$types";
import { extractRedirect, jsonBody } from "$lib/utils";
import * as set_cookie_parser from 'set-cookie-parser';
import { dev } from "$app/environment";

export const actions: Actions = {
    default: async ({ url, request, locals, cookies, fetch}) => {
        const form = await request.formData();
        const user = form.get('user') as string;
        const password = form.get('password') as string;
        //const res = await locals.api.auth.login(user, password);
        const res = await locals.api._extendResponse<string>(await locals.api.svelteFetch(locals.api.makeLoc("/auth/login"), {
            method: 'post',
            ...jsonBody({user, password})
        }))
        if (!res.api) {
            return fail(res.status, {success: false, message: await res.text()})
        }
        if (!res.api.success) {
            return fail(res.status, {success: false, message: res.api.message})
        }
        cookies.set('session_token', res.api.response, {
            httpOnly: true,
            path: '/',
            sameSite: 'strict',
            secure: !dev
        })
        throw redirect(303, extractRedirect(url.searchParams))
    }
};

export const load: PageServerLoad = async ({url, locals}) => {
    if (locals.user) {
        throw redirect(303, extractRedirect(url.searchParams))
    }
};