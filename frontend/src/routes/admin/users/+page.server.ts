import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({locals}) => {
    return {
        users: await locals.apis.users.list()
    }
};

export const actions: Actions = {
    delete: async ({request, locals}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        await locals.apis.users.delete(id);
    }
};