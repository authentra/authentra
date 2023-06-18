import { checkResponse, type Api } from "$lib/api";
import type { UserRole } from "$lib/api/types";
import { jsonBody } from "$lib/utils";

export interface AdminUser {
    id: string,
    name: string,
    email: string | null,
    active: boolean,
    roles: UserRole[],
    customer: boolean,
    require_password_reset: boolean,

}

export class UserApi {
    private api: Api;

    constructor(api: Api) {
        this.api = api
    }

    list(): Promise<AdminUser[]> {
        return checkResponse(this.api.get('/users')).then(res => res.response)
    }
    create(name: string, password: string, customer: boolean, roles: UserRole[]): Promise<void> {
        return checkResponse(this.api.post('/users', { ...jsonBody({ name, password, customer, roles }) })).then(res => res.response)
    }
    edit(id: string, name: string, email: string | null, active: boolean, roles: UserRole[], customer: boolean, require_password_reset: boolean): Promise<void> {
        return checkResponse(this.api.put('/users/' + id, {
            ...jsonBody({
                name, email, active, roles, customer, require_password_reset
            })
        })).then(res => res.response)
    }
    get(id: string): Promise<AdminUser> {
        return checkResponse(this.api.get('/users/' + id)).then(res => res.response)
    }
    delete(id: string): Promise<void> {
        return checkResponse(this.api.delete('/users/' + id)).then(res => res.response)
    }
}