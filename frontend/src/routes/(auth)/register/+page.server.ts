import { fail, redirect } from "@sveltejs/kit";
import type { Actions, PageServerLoad } from "./$types";
import { extractRedirect, jsonBody } from "$lib/utils";
import type { ApiResponse, ExtendedResponse } from "$lib/api";

export const actions: Actions = {
    default: async ({request, locals}) => {
        const form = await request.formData();
        const user = form.get('user') as string;
        const password = form.get('password') as string;
        const res = await locals.api.post('/auth/browser/register', jsonBody({user, password}))
        console.log(await res.text())
        if (!res.api) {
            return fail(res.status, {success: false, message: await res.text()})
        }
        return res.api;
    }
};

export const load: PageServerLoad = async ({url, locals}) => {
    const registrationEnabled = await locals.api.get<boolean>('/auth/registration')
    if (registrationEnabled.api && registrationEnabled.api.success && !registrationEnabled.api.response) {
        throw redirect(303, '/login')
    }
    if (locals.user) {
        throw redirect(303, extractRedirect(url.searchParams))
    }
};