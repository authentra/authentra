import { redirect } from "@sveltejs/kit";
import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({url, locals, setHeaders}) => {
    setHeaders({
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Credentials': 'true',
        'Access-Control-Allow-Methods': 'GET'
    })
    const data = await locals.apis.oauth.check(url.searchParams);
    if (data.success == 'redirect') {
        data.makeRedirect()
    }
    console.log(data)
    return {
        check: data
    }
};

export const actions: Actions = {
    authorize_next: async ({request, locals}) => {
        let formData = await request.formData();
        const searchParams = new URLSearchParams();
        for (const entry of formData.entries()) {
            searchParams.append(entry[0], entry[1] as string);
        }
        const res = await locals.apis.oauth.post(searchParams);
        if (res.success == 'redirect') {
            res.makeRedirect()
        } else {
            return res
        }
    },
    deny: async ({request, locals}) => {
        let formData = await request.formData();
        const searchParams = new URLSearchParams();
        for (const entry of formData.entries()) {
            searchParams.append(entry[0], entry[1] as string);
        }
        searchParams.append('denied', 'on');
        const res = await locals.apis.oauth.post(searchParams);
        if (res.success == 'redirect') {
            res.makeRedirect()
        } else {
            return res
        }
    }
};