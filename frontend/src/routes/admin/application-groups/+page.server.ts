import { checkAdmin, createMeta } from "$lib/server/utils";
import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({url,locals}) => {
    checkAdmin(url, locals)
    return {
        groups: await locals.admin.applicationGroups.allGroups(),
        meta: createMeta(locals)
    }
};

export const actions: Actions = {
    remove_scope: async ({request}) => {
        const formData = await request.formData();
        const id = formData.get('id');
        const scope = formData.get('scope');
        return {}
    },
    add_scope: async ({request}) => {
        const formData = await request.formData();
        const id = formData.get('id');
        const scope = formData.get('scope');
        return {}
    },
    used_by: async ({request}) => {
        const formData = await request.formData();
        const id = formData.get('id');
    }
};