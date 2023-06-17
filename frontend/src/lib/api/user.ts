import type { Api } from ".";
import type { UserRole } from "./types";

export interface User {
    name: string,
    roles: UserRole[],
    require_password_reset: boolean,
}

export class UserApi {
    private api: Api

    constructor(api: Api) {
        this.api = api;
    }

    async me(): Promise<User | null> {
        try {
            const res = await this.api.get('/users/@me');
            if (res.api && res.api.success) {
                return res.api.response
            } else {
                return null
            }
        } catch (err) {
            return null;
        }
    }
}