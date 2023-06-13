import { InternalScopes, type InternalScope } from "$lib/api/admin";
import { checkAdmin, createMeta } from "$lib/server/utils";
import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({url,locals}) => {
    return {
        groups: await locals.admin.applicationGroups.allGroups(),
        meta: createMeta(locals)
    }
};

export const actions: Actions = {
    replace: async ({locals, request}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        const scopes: InternalScope[] = [];
        console.log(JSON.stringify(formData.entries()));
        for (const [index, value] of InternalScopes.entries()) {
            const data = formData.get(value);
            if (data) {
                scopes.push(value);
            }
        }
        return await locals.admin.applicationGroups.replace(id, scopes)
    },
    used_by: async ({locals, request}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        return await locals.admin.applicationGroups.usedBy(id)
    }
};