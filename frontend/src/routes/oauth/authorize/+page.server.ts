import { redirect } from "@sveltejs/kit";
import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({url, locals}) => {
    return {
        check: await locals.apis.oauth.check(url.searchParams)
    }
};

export const actions: Actions = {
    authorize_next: async ({request, locals}) => {
        let formData = await request.formData();
        const searchParams = new URLSearchParams();
        for (const entry of formData.entries()) {
            searchParams.append(entry[0], entry[1] as string);
        }
        const res = await locals.api.post('/oauth/authorize?'+searchParams.toString(), {redirect: 'manual'}, true);
        if (res.status >= 300 && res.status <= 399) {
            //@ts-expect-error
            throw redirect(res.status, res.headers.get('location'));
        }
    },
    deny: async ({request, locals}) => {
        let formData = await request.formData();
        const searchParams = new URLSearchParams();
        for (const entry of formData.entries()) {
            searchParams.append(entry[0], entry[1] as string);
        }
        searchParams.append('denied', 'on');
        const res = await locals.api.post('/oauth/authorize?'+searchParams.toString(), {redirect: 'manual'}, true);
        if (res.status >= 300 && res.status <= 399) {
            //@ts-expect-error
            throw redirect(res.status, res.headers.get('location'));
        }
    }
};