import type { Api } from ".";

export interface User {
    name: string,
}

export class UserApi {
    private api: Api

    constructor(api: Api) {
        this.api = api;
    }

    async me(): Promise<User | null> {
        try {
            const res = await this.api.get('/user/@me');
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