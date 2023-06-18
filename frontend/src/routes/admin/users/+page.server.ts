import { getRolesFromForm } from "$lib/server/utils";
import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({locals}) => {
    return {
        users: await locals.apis.users.list()
    }
};

export const actions: Actions = {
    create: async ({request, locals}) => {
        const formData = await request.formData();
        const name = formData.get('name') as string;
        const password = formData.get('password') as string;
        const customer = formData.has('customer');
        const roles = getRolesFromForm(formData);
        return await locals.apis.users.create(name, password, customer, roles);
    },
};